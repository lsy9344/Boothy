//! Story 7.6: 렌더 우선순위 스케줄러.
//!
//! **오늘의 "렌더 큐"는 큐가 아니었다.** `acquire_render_queue_slot()`은 슬롯 2개가 다 차 있으면
//! 기다리지 않고 `render-queue-saturated`로 즉시 실패했다. 그래서 두 가지 결과가 따라왔다.
//!
//! 1. `proxyQueueWaitMicros`가 구조적으로 거의 0이었다. HV-15 evidence의 낮은 큐 대기값은
//!    "대기가 없었다"가 아니라 **"대기라는 개념이 없었다"**는 뜻이다
//! 2. 실제 위험은 대기가 아니라 **탈락**이었다. 384px 레일 정밀화와 final이 두 슬롯을 잡고 있으면
//!    고객의 첫 화면이 늦게 만들어지는 게 아니라 **아예 만들어지지 않았다**
//!
//! 이 모듈은 그 자리를 진짜 큐로 바꾼다. 작업은 우선순위·deadline과 함께 들어가고,
//! worker는 **가장 높은 우선순위 + 가장 이른 deadline**을 먼저 꺼낸다. 가득 차면 기다린다.
//!
//! **deadline은 순서를 정할 뿐 고객의 유일한 사진을 버리지 않는다.** deadline이 지났다는 이유로
//! P0를 시작하지 않거나 중단하는 경로는 이 모듈에 없다 — 실측이 8초대인 지금 그렇게 하면
//! 화면에 아무것도 뜨지 않는다. 늦어도 진실하게 띄우고 늦었다고 기록한다.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, LazyLock, Mutex};

/// 전체 동시 실행 상한. **Story 7.4의 값을 그대로 유지한다.**
pub const MAX_IN_FLIGHT_RENDER_JOBS: usize = 2;

/// 현재 촬영 display lane(P0+P1)의 동시 실행 상한.
///
/// **기본 1로 시작한다.** P0가 도는 동안 같은 촬영의 P1이 CPU를 나눠 가지면 첫 화면이 늦어지고,
/// 그것이 이 Story가 줄이려는 바로 그 구간이다.
pub const MAX_IN_FLIGHT_CURRENT_CAPTURE_RENDER_JOBS: usize = 1;

/// 작업 우선순위. 값이 작을수록 먼저 실행된다.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    /// 현재 촬영의 `displayFitPresetProxy`. **고객의 첫 성공 화면이다.**
    P0CurrentProxy,
    /// 현재 촬영의 `rawRefinedDisplay`. 승급이며 첫 화면이 아니다.
    P1CurrentRawRefined,
    /// final 렌더, 384px 레일 정밀화, preview warm-up, history 작업.
    ///
    /// **display 이벤트로 취소되지 않는다.** Story 3.2의 `Completed` / `Export Waiting` 진실이
    /// final 산출물에 달려 있다. display lane이 급하다는 이유로 final을 취소하면
    /// 고객이 결과물을 못 받는다. P2는 **뒤로 밀릴 수 있을 뿐 취소되지 않는다.**
    P2Background,
}

impl JobPriority {
    pub fn order(self) -> u8 {
        match self {
            JobPriority::P0CurrentProxy => 0,
            JobPriority::P1CurrentRawRefined => 1,
            JobPriority::P2Background => 2,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            JobPriority::P0CurrentProxy => "P0",
            JobPriority::P1CurrentRawRefined => "P1",
            JobPriority::P2Background => "P2",
        }
    }

    /// 현재 촬영 display lane인가? 이 lane만 별도의 좁은 동시 실행 상한을 갖는다.
    pub fn is_current_capture_lane(self) -> bool {
        matches!(
            self,
            JobPriority::P0CurrentProxy | JobPriority::P1CurrentRawRefined
        )
    }
}

/// 실행 중인 렌더에 실려 다니는 취소 신호.
///
/// 렌더 loop가 100 ms polling 사이에 이 값을 확인한다. 취소는 **로그와 span에만 남고
/// 고객 화면에는 나타나지 않는다** — RAW·preview·final·현재 화면 truth를 건드리지 않는다.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    reason: Arc<Mutex<Option<String>>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self, reason: &str) {
        if let Ok(mut slot) = self.reason.lock() {
            slot.get_or_insert_with(|| reason.to_string());
        }

        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn reason(&self) -> Option<String> {
        self.reason.lock().ok().and_then(|slot| slot.clone())
    }
}

/// 작업이 어느 세션·촬영·viewer 세대에 속하는지. 취소 범위 판정이 전부 이 값으로 이뤄진다.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobCoordinates {
    pub session_id: String,
    pub request_id: String,
    pub capture_id: Option<String>,
    /// **촬영 시점에 고정한** 순서 좌표. 게시 시점 좌표가 아니다.
    pub capture_order: Option<u64>,
    /// request가 수락될 때 관측한 viewer 세대.
    pub viewer_epoch: u64,
}

/// 큐에 들어가는 한 건의 요청.
#[derive(Debug, Clone)]
pub struct RenderJobRequest {
    pub priority: JobPriority,
    /// `session/request/capture/tier/presetId@version`. **같은 키는 두 번 렌더하지 않는다.**
    pub job_key: String,
    /// 촬영 시점 + NFR-003 hard max. **순서 결정과 관측에만 쓴다.**
    pub deadline_micros: u64,
    pub coordinates: JobCoordinates,
}

/// 스케줄러 구간의 진단 span. **어느 것도 KPI 종료점이 아니다.**
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchedulerSpans {
    pub enqueued_at_micros: u64,
    pub dequeued_at_micros: u64,
    /// **이제 진짜 대기 시간이다.** v3 이전 값은 승인 게이트의 mutex 시간이며 대기 시간이 아니었다.
    pub queue_wait_micros: u64,
    pub priority: &'static str,
    pub deadline_micros: u64,
    /// deadline을 넘겨서 시작했는가. **관측일 뿐 중단 사유가 아니다.**
    pub deadline_missed: bool,
    /// 같은 `job_key`로 병합된 중복 요청 수.
    pub coalesced_count: u32,
    /// 취소된 경우 그 사유. 실행된 작업에서는 `None`이다.
    pub preempted_by: Option<String>,
}

/// 큐 진입 결과.
#[derive(Debug)]
pub enum SchedulerAdmission {
    /// 슬롯을 잡았다. lease가 drop될 때 슬롯이 반납된다.
    Granted(Box<RenderLease>),
    /// 같은 `job_key`가 이미 대기·실행 중이다. **호출자는 렌더하지 않는다.**
    Coalesced {
        job_key: String,
        spans: SchedulerSpans,
    },
    /// 대기 중에 취소됐다. **렌더는 시작조차 하지 않았다.**
    Cancelled { spans: SchedulerSpans },
}

/// 실행 슬롯. drop되면 슬롯이 반납되고 대기 중인 다음 작업이 깨어난다.
#[derive(Debug)]
pub struct RenderLease {
    scheduler: Arc<RenderScheduler>,
    id: u64,
    job_key: String,
    token: CancellationToken,
    spans: SchedulerSpans,
}

impl RenderLease {
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    pub fn spans(&self) -> &SchedulerSpans {
        &self.spans
    }

    pub fn job_key(&self) -> &str {
        &self.job_key
    }
}

impl Drop for RenderLease {
    fn drop(&mut self) {
        self.scheduler.release(self.id);
    }
}

/// 취소 범위. **어느 범위가 어느 우선순위를 건드리는지가 이 enum의 전부다.**
#[derive(Debug, Clone)]
pub enum CancelScope {
    /// capture 삭제. 그 request의 **P0/P1/P2 전부** 취소한다.
    Request(String),
    /// 세션 교체. 이 세션에 속하지 않는 **전부** 취소한다.
    OtherSessions(String),
    /// viewer epoch 변경. **display lane(P0/P1)만** 취소한다.
    ///
    /// P2는 관람 창 세대와 무관한 산출물(final, 384px 레일)을 만들므로 건드리지 않는다.
    StaleViewerEpoch {
        session_id: String,
        current_epoch: u64,
    },
    /// **더 새 촬영의 generation이 pointer에 commit됐다. P1만 취소한다.**
    ///
    /// 그 뒤에는 `older-capture` guard로 어차피 거부되므로 계속 돌리는 것은 순수 낭비다.
    /// **P0는 취소하지 않는다** — 이 사진도 고객이 실제로 찍은 사진이고, 순서는 guard가 지킨다.
    SupersededRefined {
        session_id: String,
        capture_order: u64,
    },
}

impl CancelScope {
    fn label(&self) -> &'static str {
        match self {
            CancelScope::Request(_) => "request-forgotten",
            CancelScope::OtherSessions(_) => "session-replaced",
            CancelScope::StaleViewerEpoch { .. } => "viewer-epoch-changed",
            CancelScope::SupersededRefined { .. } => "superseded-by-newer-capture",
        }
    }

    fn matches(&self, priority: JobPriority, coordinates: &JobCoordinates) -> bool {
        match self {
            CancelScope::Request(request_id) => &coordinates.request_id == request_id,
            CancelScope::OtherSessions(session_id) => &coordinates.session_id != session_id,
            CancelScope::StaleViewerEpoch {
                session_id,
                current_epoch,
            } => {
                priority.is_current_capture_lane()
                    && &coordinates.session_id == session_id
                    && coordinates.viewer_epoch != *current_epoch
            }
            CancelScope::SupersededRefined {
                session_id,
                capture_order,
            } => {
                priority == JobPriority::P1CurrentRawRefined
                    && &coordinates.session_id == session_id
                    && coordinates
                        .capture_order
                        .is_some_and(|order| order < *capture_order)
            }
        }
    }
}

/// 한 번의 취소 요청이 실제로 무엇을 건드렸는지.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CancelSummary {
    /// 아직 시작하지 않아 렌더 없이 사라진 작업 수.
    pub cancelled_waiting: usize,
    /// 이미 돌고 있어 process tree 종료가 필요한 작업 수.
    pub cancelled_running: usize,
}

#[derive(Debug)]
struct WaitingJob {
    id: u64,
    priority: JobPriority,
    job_key: String,
    coordinates: JobCoordinates,
    deadline_micros: u64,
    enqueued_at_micros: u64,
    coalesced_count: u32,
    cancelled_by: Option<String>,
}

#[derive(Debug)]
struct RunningJob {
    id: u64,
    priority: JobPriority,
    job_key: String,
    coordinates: JobCoordinates,
    token: CancellationToken,
}

#[derive(Debug, Default)]
struct SchedulerInner {
    next_id: u64,
    waiting: Vec<WaitingJob>,
    running: Vec<RunningJob>,
}

impl SchedulerInner {
    fn running_total(&self) -> usize {
        self.running.len()
    }

    fn running_current_capture_lane(&self) -> usize {
        self.running
            .iter()
            .filter(|job| job.priority.is_current_capture_lane())
            .count()
    }

    /// 지금 시작할 수 있는 작업 하나를 고른다.
    ///
    /// **우선순위 → 이른 deadline → FIFO** 순으로 훑고, 자기 lane 용량이 막힌 작업은 건너뛴다.
    /// 건너뛰기가 필요한 이유: P0가 display lane 용량 때문에 대기하는 동안에도
    /// 전체 슬롯이 남아 있으면 P2가 돌 수 있어야 한다. 그러지 않으면 두 번째 슬롯이 그냥 논다.
    fn next_runnable_id(&self) -> Option<u64> {
        if self.running_total() >= MAX_IN_FLIGHT_RENDER_JOBS {
            return None;
        }

        let lane_in_flight = self.running_current_capture_lane();
        let mut ranked: Vec<&WaitingJob> = self
            .waiting
            .iter()
            .filter(|job| job.cancelled_by.is_none())
            .collect();

        ranked.sort_by(|left, right| {
            left.priority
                .order()
                .cmp(&right.priority.order())
                .then(left.deadline_micros.cmp(&right.deadline_micros))
                .then(left.id.cmp(&right.id))
        });

        ranked
            .into_iter()
            .find(|job| {
                !job.priority.is_current_capture_lane()
                    || lane_in_flight < MAX_IN_FLIGHT_CURRENT_CAPTURE_RENDER_JOBS
            })
            .map(|job| job.id)
    }
}

/// base_dir 하나당 하나. 프로세스 전역이며 테스트는 서로 다른 root로 격리된다.
#[derive(Debug, Default)]
pub struct RenderScheduler {
    inner: Mutex<SchedulerInner>,
    ready: Condvar,
}

static SCHEDULERS: LazyLock<Mutex<HashMap<PathBuf, Arc<RenderScheduler>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 이 세션 root의 스케줄러를 얻는다. 없으면 만든다.
pub fn scheduler_for(base_dir: &Path) -> Arc<RenderScheduler> {
    let mut schedulers = SCHEDULERS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    Arc::clone(
        schedulers
            .entry(base_dir.to_path_buf())
            .or_insert_with(|| Arc::new(RenderScheduler::default())),
    )
}

impl RenderScheduler {
    fn lock(&self) -> std::sync::MutexGuard<'_, SchedulerInner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 큐에 넣고 자기 차례가 될 때까지 **기다린다.**
    ///
    /// 오늘의 즉시 실패 경로를 대체하는 함수다. 가득 찼다고 고객의 첫 화면을 포기하지 않는다.
    pub fn admit(
        self: &Arc<Self>,
        request: &RenderJobRequest,
        now: &dyn Fn() -> u64,
    ) -> SchedulerAdmission {
        let enqueued_at_micros = now();
        let mut inner = self.lock();

        // 병합: 같은 좌표의 같은 작업을 두 번 렌더하지 않는다.
        if let Some(waiting) = inner.waiting.iter_mut().find(|job| {
            job.job_key == request.job_key
                && job.coordinates == request.coordinates
                && job.cancelled_by.is_none()
        }) {
            waiting.coalesced_count = waiting.coalesced_count.saturating_add(1);
            let coalesced_count = waiting.coalesced_count;

            return SchedulerAdmission::Coalesced {
                job_key: request.job_key.clone(),
                spans: SchedulerSpans {
                    enqueued_at_micros,
                    dequeued_at_micros: enqueued_at_micros,
                    queue_wait_micros: 0,
                    priority: request.priority.as_str(),
                    deadline_micros: request.deadline_micros,
                    deadline_missed: enqueued_at_micros > request.deadline_micros,
                    coalesced_count,
                    preempted_by: None,
                },
            };
        }

        if inner.running.iter().any(|job| {
            job.job_key == request.job_key
                && job.coordinates == request.coordinates
                && !job.token.is_cancelled()
        }) {
            return SchedulerAdmission::Coalesced {
                job_key: request.job_key.clone(),
                spans: SchedulerSpans {
                    enqueued_at_micros,
                    dequeued_at_micros: enqueued_at_micros,
                    queue_wait_micros: 0,
                    priority: request.priority.as_str(),
                    deadline_micros: request.deadline_micros,
                    deadline_missed: enqueued_at_micros > request.deadline_micros,
                    coalesced_count: 1,
                    preempted_by: None,
                },
            };
        }

        inner.next_id = inner.next_id.saturating_add(1);
        let id = inner.next_id;
        inner.waiting.push(WaitingJob {
            id,
            priority: request.priority,
            job_key: request.job_key.clone(),
            coordinates: request.coordinates.clone(),
            deadline_micros: request.deadline_micros,
            enqueued_at_micros,
            coalesced_count: 0,
            cancelled_by: None,
        });

        // 새 작업이 들어오면 순위가 바뀔 수 있다. 대기 중인 다른 작업도 다시 판정하게 한다.
        self.ready.notify_all();

        loop {
            let position = inner
                .waiting
                .iter()
                .position(|job| job.id == id)
                .expect("enqueued job must stay in the waiting list until it is taken out");

            if let Some(reason) = inner.waiting[position].cancelled_by.clone() {
                let job = inner.waiting.remove(position);
                let cancelled_at_micros = now();

                return SchedulerAdmission::Cancelled {
                    spans: SchedulerSpans {
                        enqueued_at_micros: job.enqueued_at_micros,
                        dequeued_at_micros: cancelled_at_micros,
                        queue_wait_micros: cancelled_at_micros
                            .saturating_sub(job.enqueued_at_micros),
                        priority: job.priority.as_str(),
                        deadline_micros: job.deadline_micros,
                        deadline_missed: cancelled_at_micros > job.deadline_micros,
                        coalesced_count: job.coalesced_count,
                        preempted_by: Some(reason),
                    },
                };
            }

            if inner.next_runnable_id() == Some(id) {
                let job = inner.waiting.remove(position);
                let dequeued_at_micros = now();
                let token = CancellationToken::new();

                inner.running.push(RunningJob {
                    id: job.id,
                    priority: job.priority,
                    job_key: job.job_key.clone(),
                    coordinates: job.coordinates.clone(),
                    token: token.clone(),
                });

                return SchedulerAdmission::Granted(Box::new(RenderLease {
                    scheduler: Arc::clone(self),
                    id: job.id,
                    job_key: job.job_key,
                    token,
                    spans: SchedulerSpans {
                        enqueued_at_micros: job.enqueued_at_micros,
                        dequeued_at_micros,
                        queue_wait_micros: dequeued_at_micros
                            .saturating_sub(job.enqueued_at_micros),
                        priority: job.priority.as_str(),
                        deadline_micros: job.deadline_micros,
                        deadline_missed: dequeued_at_micros > job.deadline_micros,
                        coalesced_count: job.coalesced_count,
                        preempted_by: None,
                    },
                }));
            }

            inner = self
                .ready
                .wait(inner)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }

    /// 렌더러가 완전히 놀고 있을 때만 슬롯을 준다. 그렇지 않으면 즉시 포기한다.
    ///
    /// preview warm-up 전용이다. **warm-up은 실패해도 제품 진실에 영향이 없으므로**
    /// 줄을 서지 않는다 — 서면 고객의 첫 화면 뒤에 불필요한 darktable 실행이 하나 더 붙는다.
    pub fn try_admit_idle_background(
        self: &Arc<Self>,
        request: &RenderJobRequest,
        now: &dyn Fn() -> u64,
    ) -> Option<RenderLease> {
        let at_micros = now();
        let mut inner = self.lock();

        if !inner.running.is_empty() || !inner.waiting.is_empty() {
            return None;
        }

        inner.next_id = inner.next_id.saturating_add(1);
        let id = inner.next_id;
        let token = CancellationToken::new();

        inner.running.push(RunningJob {
            id,
            priority: request.priority,
            job_key: request.job_key.clone(),
            coordinates: request.coordinates.clone(),
            token: token.clone(),
        });

        Some(RenderLease {
            scheduler: Arc::clone(self),
            id,
            job_key: request.job_key.clone(),
            token,
            spans: SchedulerSpans {
                enqueued_at_micros: at_micros,
                dequeued_at_micros: at_micros,
                queue_wait_micros: 0,
                priority: request.priority.as_str(),
                deadline_micros: request.deadline_micros,
                deadline_missed: false,
                coalesced_count: 0,
                preempted_by: None,
            },
        })
    }

    /// 범위에 드는 작업을 취소한다. 대기 중이면 큐에서, 실행 중이면 token으로.
    ///
    /// **실행 중인 작업의 process tree 종료는 렌더 loop가 한다.** 이 함수는 신호만 보낸다 —
    /// 스케줄러 mutex를 잡은 채로 `taskkill`을 기다리면 그동안 큐 전체가 멈춘다.
    pub fn cancel(&self, scope: &CancelScope) -> CancelSummary {
        let reason = scope.label();
        let mut summary = CancelSummary::default();

        {
            let mut inner = self.lock();

            for job in inner.waiting.iter_mut() {
                if job.cancelled_by.is_none() && scope.matches(job.priority, &job.coordinates) {
                    job.cancelled_by = Some(reason.to_string());
                    summary.cancelled_waiting += 1;
                }
            }

            for job in inner.running.iter() {
                if !job.token.is_cancelled() && scope.matches(job.priority, &job.coordinates) {
                    job.token.cancel(reason);
                    summary.cancelled_running += 1;
                }
            }
        }

        self.ready.notify_all();
        summary
    }

    fn release(&self, id: u64) {
        {
            let mut inner = self.lock();
            inner.running.retain(|job| job.id != id);
        }

        self.ready.notify_all();
    }

    /// 진단용 현재 깊이. 테스트와 로그에서만 쓴다.
    pub fn depth(&self) -> (usize, usize) {
        let inner = self.lock();

        (inner.waiting.len(), inner.running.len())
    }
}

/// 병합 키. **한 촬영·한 tier·한 preset 버전당 하나의 렌더**를 강제한다.
pub fn display_job_key(
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    tier: &str,
    preset_id: &str,
    preset_version: &str,
) -> String {
    format!("{session_id}/{request_id}/{capture_id}/{tier}/{preset_id}@{preset_version}")
}

/// 같은 촬영이라도 viewer 세대·목표 geometry가 다르면 별도 렌더여야 한다.
///
/// base key는 evidence에서 읽기 쉬운 좌표를 유지하고, coalescing에만 쓰는 viewer 문맥을
/// 뒤에 붙인다. 취소 중인 이전 epoch 작업이나 resize 전 작업이 새 복구 렌더를 흡수하면 안 된다.
#[allow(clippy::too_many_arguments)]
pub fn display_job_key_for_context(
    session_id: &str,
    request_id: &str,
    capture_id: &str,
    tier: &str,
    preset_id: &str,
    preset_version: &str,
    viewer_epoch: u64,
    target_width_px: u32,
    target_height_px: u32,
    display_profile_id: &str,
    device_pixel_ratio: f64,
) -> String {
    format!(
        "{}#viewer={viewer_epoch}:{target_width_px}x{target_height_px}:{display_profile_id}:dpr={:016x}",
        display_job_key(
            session_id,
            request_id,
            capture_id,
            tier,
            preset_id,
            preset_version,
        ),
        device_pixel_ratio.to_bits()
    )
}

/// P2 작업의 병합 키. tier가 없으므로 stage 이름을 쓴다.
pub fn background_job_key(session_id: &str, capture_id: &str, stage: &str) -> String {
    format!("{session_id}/{capture_id}/background/{stage}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn clock(value: u64) -> impl Fn() -> u64 {
        move || value
    }

    fn coordinates(capture_id: &str, capture_order: u64) -> JobCoordinates {
        JobCoordinates {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            request_id: format!("req-{capture_id}"),
            capture_id: Some(capture_id.into()),
            capture_order: Some(capture_order),
            viewer_epoch: 3,
        }
    }

    fn request(priority: JobPriority, capture_id: &str, deadline: u64) -> RenderJobRequest {
        RenderJobRequest {
            priority,
            job_key: format!("{}/{}", priority.as_str(), capture_id),
            deadline_micros: deadline,
            coordinates: coordinates(capture_id, 0),
        }
    }

    fn granted(admission: SchedulerAdmission) -> RenderLease {
        match admission {
            SchedulerAdmission::Granted(lease) => *lease,
            other => panic!("expected a granted lease, got {other:?}"),
        }
    }

    #[test]
    fn the_current_capture_lane_never_runs_two_renders_at_once() {
        let scheduler = Arc::new(RenderScheduler::default());
        let p0 = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));

        // 같은 촬영의 P1은 P0가 끝날 때까지 시작하지 않는다.
        let (sender, receiver) = mpsc::channel();
        let lane_scheduler = Arc::clone(&scheduler);
        let worker = thread::spawn(move || {
            let lease = granted(lane_scheduler.admit(
                &request(JobPriority::P1CurrentRawRefined, "cap-1", 5_000_000),
                &clock(2_000),
            ));
            let _ = sender.send(());
            drop(lease);
        });

        assert!(
            receiver.recv_timeout(Duration::from_millis(150)).is_err(),
            "현재 촬영 lane 동시 실행이 1을 넘었다"
        );

        drop(p0);
        worker.join().expect("worker thread");
        assert_eq!(scheduler.depth(), (0, 0));
    }

    #[test]
    fn a_waiting_p0_runs_before_a_waiting_p2() {
        let scheduler = Arc::new(RenderScheduler::default());
        // 두 슬롯을 P2 두 건이 잡고 있다.
        let first = granted(scheduler.admit(
            &request(JobPriority::P2Background, "cap-final", 9_000_000),
            &clock(1_000),
        ));
        let second = granted(scheduler.admit(
            &request(JobPriority::P2Background, "cap-preview", 9_000_000),
            &clock(1_000),
        ));

        let order = Arc::new(Mutex::new(Vec::<&'static str>::new()));
        let mut workers = Vec::new();

        // 늦게 들어온 P2가 먼저 줄을 서 있어도 P0가 먼저 나간다.
        for (priority, label, delay_ms) in [
            (JobPriority::P2Background, "p2", 0),
            (JobPriority::P0CurrentProxy, "p0", 60),
        ] {
            let scheduler = Arc::clone(&scheduler);
            let order = Arc::clone(&order);
            workers.push(thread::spawn(move || {
                thread::sleep(Duration::from_millis(delay_ms));
                let lease = granted(
                    scheduler.admit(&request(priority, "cap-queued", 5_000_000), &clock(2_000)),
                );
                order.lock().expect("order lock").push(label);
                drop(lease);
            }));
        }

        // 두 대기자가 모두 큐에 들어갈 시간을 준 뒤에야 슬롯을 연다.
        thread::sleep(Duration::from_millis(150));
        drop(first);
        drop(second);

        for worker in workers {
            worker.join().expect("worker thread");
        }

        assert_eq!(*order.lock().expect("order lock"), vec!["p0", "p2"]);
    }

    #[test]
    fn the_earliest_deadline_goes_first_inside_one_priority() {
        let scheduler = Arc::new(RenderScheduler::default());
        let blocker = granted(scheduler.admit(
            &request(JobPriority::P2Background, "cap-block-1", 9_000_000),
            &clock(1_000),
        ));
        let second_blocker = granted(scheduler.admit(
            &request(JobPriority::P2Background, "cap-block-2", 9_000_000),
            &clock(1_000),
        ));

        let order = Arc::new(Mutex::new(Vec::<u64>::new()));
        let mut workers = Vec::new();

        // 늦은 deadline이 먼저 큐에 들어간다. FIFO만 보면 이 순서가 유지된다.
        for (deadline, delay_ms) in [(8_000_000_u64, 0_u64), (4_000_000, 60)] {
            let scheduler = Arc::clone(&scheduler);
            let order = Arc::clone(&order);
            workers.push(thread::spawn(move || {
                thread::sleep(Duration::from_millis(delay_ms));
                let lease = granted(scheduler.admit(
                    &RenderJobRequest {
                        priority: JobPriority::P2Background,
                        job_key: format!("deadline/{deadline}"),
                        deadline_micros: deadline,
                        coordinates: coordinates("cap-deadline", 0),
                    },
                    &clock(2_000),
                ));
                order.lock().expect("order lock").push(deadline);
                drop(lease);
            }));
        }

        thread::sleep(Duration::from_millis(150));
        drop(blocker);
        drop(second_blocker);

        for worker in workers {
            worker.join().expect("worker thread");
        }

        assert_eq!(
            *order.lock().expect("order lock"),
            vec![4_000_000, 8_000_000],
            "같은 우선순위에서는 이른 deadline이 먼저 나가야 한다"
        );
    }

    #[test]
    fn a_duplicate_job_key_is_merged_instead_of_rendered_twice() {
        let scheduler = Arc::new(RenderScheduler::default());
        let running = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));

        // 실행 중인 같은 작업이 다시 요청됐다.
        let merged = scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_500),
        );

        match merged {
            SchedulerAdmission::Coalesced { job_key, spans } => {
                assert_eq!(job_key, "P0/cap-1");
                assert_eq!(spans.coalesced_count, 1);
            }
            other => panic!("중복 요청은 병합돼야 한다: {other:?}"),
        }

        drop(running);
    }

    #[test]
    fn a_duplicate_waiting_job_key_is_merged_and_counted() {
        let scheduler = Arc::new(RenderScheduler::default());
        let blocker = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));

        let waiting_scheduler = Arc::clone(&scheduler);
        let waiter = thread::spawn(move || {
            granted(waiting_scheduler.admit(
                &request(JobPriority::P1CurrentRawRefined, "cap-1", 5_000_000),
                &clock(1_100),
            ))
        });

        // 대기자가 큐에 들어갈 때까지 기다린 뒤 같은 키로 다시 요청한다.
        let mut merged = None;
        for _ in 0..100 {
            if scheduler.depth().0 == 1 {
                merged = Some(scheduler.admit(
                    &request(JobPriority::P1CurrentRawRefined, "cap-1", 5_000_000),
                    &clock(1_200),
                ));
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        match merged.expect("대기 중인 작업을 관측하지 못했다") {
            SchedulerAdmission::Coalesced { spans, .. } => assert_eq!(spans.coalesced_count, 1),
            other => panic!("대기 중 중복 요청은 병합돼야 한다: {other:?}"),
        }

        drop(blocker);
        let lease = waiter.join().expect("waiter thread");
        assert_eq!(lease.spans().coalesced_count, 1);
    }

    #[test]
    fn a_missed_deadline_is_recorded_but_never_cancels_the_customers_first_frame() {
        let scheduler = Arc::new(RenderScheduler::default());
        // 촬영 시점 + 5초 예산인데 이미 8초가 지났다. 오늘의 실측이 정확히 이 상황이다.
        let lease = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(8_284_000),
        ));

        assert!(lease.spans().deadline_missed);
        assert!(!lease.token().is_cancelled());
    }

    #[test]
    fn deleting_a_capture_cancels_every_priority_of_that_request() {
        let scheduler = Arc::new(RenderScheduler::default());
        let proxy = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));
        let background = granted(scheduler.admit(
            &request(JobPriority::P2Background, "cap-1", 9_000_000),
            &clock(1_000),
        ));

        let summary = scheduler.cancel(&CancelScope::Request("req-cap-1".into()));

        assert_eq!(summary.cancelled_running, 2);
        assert!(proxy.token().is_cancelled());
        assert!(background.token().is_cancelled());
        assert_eq!(
            background.token().reason().as_deref(),
            Some("request-forgotten")
        );
    }

    /// **Story 3.2의 완료 진실이 final 산출물에 달려 있다.**
    /// display lane이 급하다는 이유로 final을 취소하면 고객이 결과물을 못 받는다.
    #[test]
    fn display_events_never_cancel_a_background_render() {
        let scheduler = Arc::new(RenderScheduler::default());
        let final_render = granted(scheduler.admit(
            &RenderJobRequest {
                priority: JobPriority::P2Background,
                job_key: "final/cap-1".into(),
                deadline_micros: 9_000_000,
                coordinates: coordinates("cap-1", 0),
            },
            &clock(1_000),
        ));

        let epoch_change = scheduler.cancel(&CancelScope::StaleViewerEpoch {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            current_epoch: 9,
        });
        let newer_capture = scheduler.cancel(&CancelScope::SupersededRefined {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            capture_order: 7,
        });

        assert_eq!(epoch_change.cancelled_running, 0);
        assert_eq!(newer_capture.cancelled_running, 0);
        assert!(!final_render.token().is_cancelled());
    }

    /// **P0는 더 새 촬영이 왔다고 취소되지 않는다.**
    /// 이 사진도 고객이 실제로 찍은 사진이고, 순서는 `older-capture` guard가 지킨다.
    #[test]
    fn a_newer_capture_cancels_only_the_older_refined_job() {
        let scheduler = Arc::new(RenderScheduler::default());
        // 정밀본이 먼저 슬롯을 잡았고, 같은 촬영의 proxy가 lane 용량을 기다린다.
        let older_refined = granted(scheduler.admit(
            &RenderJobRequest {
                priority: JobPriority::P1CurrentRawRefined,
                job_key: "refined/cap-1".into(),
                deadline_micros: 5_000_000,
                coordinates: coordinates("cap-1", 0),
            },
            &clock(1_000),
        ));

        let proxy_scheduler = Arc::clone(&scheduler);
        let proxy_worker = thread::spawn(move || {
            granted(proxy_scheduler.admit(
                &RenderJobRequest {
                    priority: JobPriority::P0CurrentProxy,
                    job_key: "proxy/cap-1".into(),
                    deadline_micros: 5_000_000,
                    coordinates: coordinates("cap-1", 0),
                },
                &clock(1_100),
            ))
        });

        for _ in 0..100 {
            if scheduler.depth().0 == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        let summary = scheduler.cancel(&CancelScope::SupersededRefined {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            capture_order: 1,
        });

        // 돌고 있던 정밀본만 취소된다. 대기 중인 P0는 건드리지 않는다 —
        // 이 사진도 고객이 실제로 찍은 사진이고, 순서는 `older-capture` guard가 지킨다.
        assert_eq!(summary.cancelled_running, 1);
        assert_eq!(summary.cancelled_waiting, 0);
        assert!(older_refined.token().is_cancelled());

        drop(older_refined);
        let proxy_lease = proxy_worker.join().expect("proxy worker thread");
        assert!(!proxy_lease.token().is_cancelled());
    }

    #[test]
    fn a_cancelled_waiting_job_never_starts_rendering() {
        let scheduler = Arc::new(RenderScheduler::default());
        let blocker = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));

        let waiting_scheduler = Arc::clone(&scheduler);
        let waiter = thread::spawn(move || {
            waiting_scheduler.admit(
                &RenderJobRequest {
                    priority: JobPriority::P1CurrentRawRefined,
                    job_key: "refined/cap-1".into(),
                    deadline_micros: 5_000_000,
                    coordinates: coordinates("cap-1", 0),
                },
                &clock(1_100),
            )
        });

        for _ in 0..100 {
            if scheduler.depth().0 == 1 {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }

        let summary = scheduler.cancel(&CancelScope::SupersededRefined {
            session_id: "session_01hs6n1r8b8zc5v4ey2x7b9g1m".into(),
            capture_order: 1,
        });
        assert_eq!(summary.cancelled_waiting, 1);

        match waiter.join().expect("waiter thread") {
            SchedulerAdmission::Cancelled { spans } => {
                assert_eq!(
                    spans.preempted_by.as_deref(),
                    Some("superseded-by-newer-capture")
                );
            }
            other => panic!("취소된 대기 작업은 렌더로 넘어가면 안 된다: {other:?}"),
        }

        drop(blocker);
    }

    #[test]
    fn a_session_change_cancels_only_the_previous_sessions_work() {
        let scheduler = Arc::new(RenderScheduler::default());
        let previous = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));
        let current = granted(scheduler.admit(
            &RenderJobRequest {
                priority: JobPriority::P2Background,
                job_key: "final/cap-2".into(),
                deadline_micros: 9_000_000,
                coordinates: JobCoordinates {
                    session_id: "session_01haaaaaaaaaaaaaaaaaaaaaaa".into(),
                    request_id: "req-cap-2".into(),
                    capture_id: Some("cap-2".into()),
                    capture_order: Some(0),
                    viewer_epoch: 3,
                },
            },
            &clock(1_000),
        ));

        let summary = scheduler.cancel(&CancelScope::OtherSessions(
            "session_01haaaaaaaaaaaaaaaaaaaaaaa".into(),
        ));

        assert_eq!(summary.cancelled_running, 1);
        assert!(previous.token().is_cancelled());
        assert!(!current.token().is_cancelled());
    }

    #[test]
    fn warm_up_gives_up_immediately_instead_of_queueing_behind_the_customer() {
        let scheduler = Arc::new(RenderScheduler::default());
        let busy = granted(scheduler.admit(
            &request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000),
            &clock(1_000),
        ));

        assert!(scheduler
            .try_admit_idle_background(
                &request(JobPriority::P2Background, "warmup", 9_000_000),
                &clock(1_100)
            )
            .is_none());

        drop(busy);

        assert!(scheduler
            .try_admit_idle_background(
                &request(JobPriority::P2Background, "warmup", 9_000_000),
                &clock(1_200)
            )
            .is_some());
    }

    #[test]
    fn a_cancelled_running_job_does_not_absorb_its_replacement() {
        let scheduler = Arc::new(RenderScheduler::default());
        let original_request = request(JobPriority::P0CurrentProxy, "cap-1", 5_000_000);
        let original = granted(scheduler.admit(&original_request, &clock(1_000)));
        let summary = scheduler.cancel(&CancelScope::Request(
            original_request.coordinates.request_id.clone(),
        ));
        assert_eq!(summary.cancelled_running, 1);

        let replacement_scheduler = Arc::clone(&scheduler);
        let replacement_request = original_request.clone();
        let replacement =
            thread::spawn(move || replacement_scheduler.admit(&replacement_request, &clock(2_000)));
        thread::sleep(Duration::from_millis(20));

        assert_eq!(scheduler.depth(), (1, 1));
        drop(original);
        assert!(matches!(
            replacement.join().expect("replacement waiter"),
            SchedulerAdmission::Granted(_)
        ));
    }

    #[test]
    fn the_capacity_constants_are_pinned() {
        // 두 값 모두 제품 결정이다. 조용히 바뀌면 첫 화면 지연이 조용히 늘어난다.
        assert_eq!(MAX_IN_FLIGHT_RENDER_JOBS, 2);
        assert_eq!(MAX_IN_FLIGHT_CURRENT_CAPTURE_RENDER_JOBS, 1);
    }

    #[test]
    fn job_keys_separate_tiers_of_the_same_capture() {
        let proxy = display_job_key(
            "session-1",
            "req-1",
            "cap-1",
            "displayFitPresetProxy",
            "preset_soft-glow",
            "2026.08.01",
        );
        let refined = display_job_key(
            "session-1",
            "req-1",
            "cap-1",
            "rawRefinedDisplay",
            "preset_soft-glow",
            "2026.08.01",
        );

        assert_ne!(proxy, refined, "두 tier가 서로를 병합해 버리면 안 된다");
        assert!(proxy.contains("preset_soft-glow@2026.08.01"));
    }

    #[test]
    fn display_job_keys_separate_viewer_geometry_and_epoch() {
        let first = display_job_key_for_context(
            "session-1",
            "req-1",
            "cap-1",
            "displayFitPresetProxy",
            "preset_soft-glow",
            "2026.08.01",
            3,
            1620,
            1080,
            "approved-1080p",
            1.0,
        );
        let resized = display_job_key_for_context(
            "session-1",
            "req-1",
            "cap-1",
            "displayFitPresetProxy",
            "preset_soft-glow",
            "2026.08.01",
            3,
            1440,
            960,
            "approved-1080p",
            1.0,
        );
        let recreated = display_job_key_for_context(
            "session-1",
            "req-1",
            "cap-1",
            "displayFitPresetProxy",
            "preset_soft-glow",
            "2026.08.01",
            4,
            1620,
            1080,
            "approved-1080p",
            1.0,
        );

        assert_ne!(first, resized);
        assert_ne!(first, recreated);
    }
}
