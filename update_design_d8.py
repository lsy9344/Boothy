import os
import re

html_path = r'c:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-directions.html'
md_path = r'c:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md'

with open(html_path, 'r', encoding='utf-8') as f:
    html = f.read()

new_direction = """
      ,{
        id: "d8",
        nav: "08 Discover",
        number: "08",
        title: "Discover Bazaar",
        accent: "#ff90e8",
        soft: "#f4f1ff",
        shape: "rounded",
        tags: ["Gumroad Discover", "다이내믹 그리드", "콘텐츠 중심 브루탈리즘"],
        summary: "Gumroad Discover 페이지의 자유롭고 활기찬 마켓플레이스 감성을 차용했습니다. 정형화된 틀을 벗어나 콘텐츠(사진) 자체가 돋보이는 레이아웃과 생동감 넘치는 반응형 인터랙션을 제공합니다.",
        bestFor: "사용자가 탐색 과정에서 재미와 생동감을 느끼길 원할 때. 촬영 결과물이 스티커북이나 매거진처럼 다채롭게 보이길 원할 때.",
        layout: "board",
        customerTitle: "내 사진이 돋보이는 활기찬 마켓.",
        customerText: "자유로운 배치, 강렬한 호버 액션, 그리고 생생한 색감 퍼포먼스. 부스가 한 편의 역동적인 디지털 쇼룸이 됩니다.",
        ready: "모든 준비 완료, 촬영 시작!",
        waiting: "활력 넘치는 대기 상태 유지",
        rail: ["비대칭형 카드", "오프셋(Offset) 그림자 버튼", "오버사이즈 타이포그래피", "자유분방한 리듬"],
        operatorTitle: "다이내믹하지만 명확한 운영 뷰",
        operatorText: "Discover의 자유분방함을 운영면에도 적용하여, 에러나 지연 상황을 지루하지 않고 명확한 블록으로 시각화합니다.",
        metrics: [["status", "active"], ["load", "low"], ["vibes", "high"]]
      }
"""

if 'id: "d8"' not in html:
    html = html.replace('metrics: [["camera", "ok"], ["delay", "03s"], ["tone", "hard"]]\n      }', 'metrics: [["camera", "ok"], ["delay", "03s"], ["tone", "hard"]]\n      }' + new_direction)
    with open(html_path, 'w', encoding='utf-8') as f:
        f.write(html)
    print("Added D8 to HTML")
else:
    print("D8 already in HTML")

with open(md_path, 'r', encoding='utf-8') as f:
    md = f.read()


new_md_direction = """
      - **08 Discover Bazaar:** Gumroad Discover 페이지의 자유롭고 활기찬 마켓플레이스 감성을 차용한 방향. 비대칭적이고 자유분방한 그리드와 강렬한 호버 액션으로 콘텐츠를 강조합니다.
"""

if '08 Discover Bazaar:' not in md:
    md = md.replace('최종 프리미엄 안입니다.', '최종 프리미엄 안입니다.' + new_md_direction)
    with open(md_path, 'w', encoding='utf-8') as f:
        f.write(md)
    print("Added D8 to MD")
else:
    print("D8 already in MD")
