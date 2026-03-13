import os
import re

html_path = r'c:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-directions.html'
md_path = r'c:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md'

with open(html_path, 'r', encoding='utf-8') as f:
    html = f.read()

# Replace Root CSS
html = re.sub(r':root\s*\{[^}]+\}', """:root {
      --bg: #09090b;
      --surface: rgba(255, 255, 255, 0.04);
      --surface-2: rgba(255, 255, 255, 0.07);
      --panel: rgba(15, 15, 20, 0.6);
      --ink: #ffffff;
      --ink-soft: #a1a1aa;
      --line: rgba(255, 255, 255, 0.1);
      --accent: #8b5cf6;
      --accent-glow: rgba(139, 92, 246, 0.4);
      --pink: #ff7eb3;
      --clay: #e48c6c;
      --butter: #fcd34d;
      --sage: #6ee7b7;
      --sky: #7dd3fc;
      --max: 1320px;
      --radius-xl: 32px;
      --radius-lg: 24px;
      --radius-md: 18px;
      --shadow-sm: 0 4px 12px rgba(0, 0, 0, 0.3);
      --shadow-md: 0 8px 24px rgba(0, 0, 0, 0.4);
      --shadow-lg: 0 16px 48px rgba(0, 0, 0, 0.5);
      --aura-gradient: radial-gradient(circle at 50% 50%, var(--accent-glow) 0%, transparent 60%);
    }""", html)

html = re.sub(r'body\s*\{\s*margin[\s\S]*?body::before\s*\{[\s\S]*?\}', """body {
      margin: 0;
      font-family: "Pretendard Variable", "Pretendard", "Noto Sans KR", sans-serif;
      color: var(--ink);
      background-color: var(--bg);
      background-image: 
        radial-gradient(circle at 15% 10%, rgba(139, 92, 246, 0.15), transparent 40%),
        radial-gradient(circle at 85% 85%, rgba(255, 126, 179, 0.15), transparent 40%);
    }

    body::before {
      content: "";
      position: fixed;
      inset: 0;
      pointer-events: none;
      background: url('data:image/svg+xml;utf8,<svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg"><filter id="noiseFilter"><feTurbulence type="fractalNoise" baseFrequency="0.8" numOctaves="3" stitchTiles="stitch"/></filter><rect width="100%" height="100%" filter="url(#noiseFilter)" opacity="0.04"/></svg>');
      z-index: 9999;
      opacity: 0.6;
    }""", html)

html = re.sub(r'\.site-header\s*\{[\s\S]*?\}', """.site-header {
      position: sticky;
      top: 0;
      z-index: 20;
      background: rgba(9, 9, 11, 0.6);
      border-bottom: 1px solid var(--line);
      backdrop-filter: blur(20px);
      -webkit-backdrop-filter: blur(20px);
    }""", html)

html = re.sub(r'\.brand-badge\s*\{[\s\S]*?\}', """.brand-badge {
      width: 42px;
      height: 42px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border: 1px solid rgba(255,255,255,0.2);
      border-radius: 999px;
      background: linear-gradient(135deg, var(--accent), var(--pink));
      box-shadow: 0 0 20px var(--accent-glow);
      font-weight: 800;
      color: #fff;
    }""", html)

html = html.replace('.nav-pill,\n    .tag,\n    .mini-pill,\n    .status-pill,\n    .pick-chip,\n    .compare-clear,\n    .choose-button {\n      display: inline-flex;\n      align-items: center;\n      justify-content: center;\n      border: 2px solid var(--line);\n      border-radius: 999px;\n      background: #fff;\n      min-height: 42px;\n      padding: 0 16px;\n      font-size: 0.94rem;\n      font-weight: 600;\n      transition: transform 120ms ease, box-shadow 120ms ease, background 120ms ease;\n      box-shadow: var(--shadow-sm);\n    }', """.nav-pill,
    .tag,
    .mini-pill,
    .status-pill,
    .pick-chip,
    .compare-clear,
    .choose-button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border: 1px solid var(--line);
      border-radius: 999px;
      background: var(--surface);
      backdrop-filter: blur(10px);
      min-height: 42px;
      padding: 0 16px;
      font-size: 0.94rem;
      font-weight: 600;
      color: var(--ink-soft);
      transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    }""")

html = html.replace('.nav-pill:hover,\n    .choose-button:hover,\n    .compare-clear:hover {\n      transform: translate(-2px, -2px);\n      box-shadow: var(--shadow-md);\n    }', """.nav-pill:hover,
    .choose-button:hover,
    .compare-clear:hover {
      background: var(--surface-2);
      color: var(--ink);
      border-color: rgba(255,255,255,0.3);
      box-shadow: 0 4px 20px rgba(0,0,0,0.5);
    }""")

html = html.replace('.nav-pill.active {\n      background: var(--ink);\n      color: #fff8ef;\n    }', """.nav-pill.active {
      background: #fff;
      color: #000;
      border-color: #fff;
      box-shadow: 0 0 20px rgba(255,255,255,0.2);
    }""")

html = html.replace('.header-button {\n      min-height: 48px;\n      padding: 0 18px;\n      border: 2px solid var(--line);\n      border-radius: 999px;\n      background: #fff;\n      box-shadow: var(--shadow-sm);\n      font-weight: 600;\n      transition: transform 120ms ease, box-shadow 120ms ease, background 120ms ease;\n    }', """.header-button {
      min-height: 48px;
      padding: 0 18px;
      border: 1px solid var(--line);
      border-radius: 999px;
      background: var(--surface);
      backdrop-filter: blur(10px);
      color: var(--ink);
      font-weight: 600;
      transition: all 0.3s ease;
    }""")

html = html.replace('.header-button:hover {\n      transform: translate(-2px, -2px);\n      box-shadow: var(--shadow-md);\n    }', """.header-button:hover {
      background: var(--surface-2);
      border-color: rgba(255,255,255,0.3);
    }""")

html = html.replace('.header-button.primary {\n      background: var(--ink);\n      color: #fff8ef;\n    }', """.header-button.primary {
      background: #fff;
      color: #000;
      border-color: #fff;
    }""")

html = html.replace('.hero {\n      border: 2px solid var(--line);\n      border-radius: 40px;\n      overflow: hidden;\n      background: var(--paper-2);\n      box-shadow: var(--shadow-lg);\n      display: grid;\n      grid-template-columns: 1.25fr 0.75fr;\n    }', """.hero {
      border: 1px solid var(--line);
      border-radius: 40px;
      overflow: hidden;
      background: rgba(15, 15, 20, 0.4);
      backdrop-filter: blur(20px);
      box-shadow: var(--shadow-lg), inset 0 0 0 1px rgba(255,255,255,0.05);
      display: grid;
      grid-template-columns: 1.25fr 0.75fr;
      position: relative;
    }""")

html = html.replace('.primary-button,\n    .secondary-button {\n      min-height: 58px;\n      padding: 0 22px;\n      border: 2px solid var(--line);\n      border-radius: 999px;\n      box-shadow: var(--shadow-md);\n      font-size: 1rem;\n      font-weight: 700;\n      transition: transform 120ms ease, box-shadow 120ms ease;\n    }', """.primary-button,
    .secondary-button {
      min-height: 58px;
      padding: 0 28px;
      border: 1px solid rgba(255,255,255,0.1);
      border-radius: 999px;
      font-size: 1.05rem;
      font-weight: 700;
      transition: all 0.4s cubic-bezier(0.16, 1, 0.3, 1);
      backdrop-filter: blur(10px);
      display: inline-flex;
      align-items: center;
      justify-content: center;
    }""")

html = html.replace('.primary-button { background: var(--ink); color: #fff8ef; }', '.primary-button { background: #fff; color: #000; }')
html = html.replace('.secondary-button { background: #fff; color: var(--ink); }', '.secondary-button { background: var(--surface); color: var(--ink); }')
html = html.replace('.primary-button:hover,\n    .secondary-button:hover {\n      transform: translate(-3px, -3px);\n      box-shadow: var(--shadow-lg);\n    }', """.primary-button:hover {
      box-shadow: 0 0 30px rgba(255,255,255,0.3);
      transform: translateY(-2px);
    }
    .secondary-button:hover {
      background: var(--surface-2);
      border-color: rgba(255,255,255,0.3);
      transform: translateY(-2px);
    }""")

html = html.replace('Gumroad-Style Directions', 'Sensuous Aura Directions')
html = html.replace('Gumroad Reset', 'Sensuous Aura')
html = html.replace('Style reset / delete old direction / rebuild from Gumroad', 'Sensuous redesign / Cinematic depth / Aura blur')
html = html.replace('Boothy를 Gumroad의 뼈대로 다시 설계했습니다.', 'Boothy를 감각적이고 압도적인 시네마틱 경험으로 재조율했습니다.')
html = html.replace('이전 쇼케이스 구조는 기준에서 벗어나 있었습니다. 이번 버전은 Gumroad의 현재 공식 홈페이지에서 읽히는 시각 문법을 우선으로 두고 다시 만들었습니다. 핵심은 초대형 헤드라인, 검은 윤곽선, 둥근 필 버튼, 종이 같은 따뜻한 표면, 그리고 브랜드가 먼저 들어오는 편집적 자신감입니다.', '가장 직관적인 구조 위에 완전히 새로운 심미성을 얹었습니다. 깊은 어둠 속에서 발광하는 오라(Aura), 빛을 투과하는 글래스모피즘(Glassmorphism), 그리고 시네마틱한 대비가 공간 전체의 분위기를 지배합니다.')
html = html.replace('Gumroad style first', 'Cinematic mood')
html = html.replace('warm paper surfaces', 'Glassmorphism layers')
html = html.replace('black outlines + pill controls', 'Glow aura & delicate lines')
html = html.replace('네오브루탈리즘 일반론을 버리고, Gumroad식 warm outlined editorial language를 Boothy에 맞게 재해석합니다.', '너무 일반적이었던 기존 스타일을 버리고, 다크 모드 기반의 프리미엄 하이엔드 미학으로 완전히 재해석합니다.')

html = html.replace('1. 한 가족 폰트', '1. 세련된 타이포그래피')
html = html.replace('Pretendard Variable 하나로 제목과 UI를 같이 운영합니다.', 'Pretendard의 우아한 자간과 얇고 세련된 웨이트를 조합하여 고급스러움을 극대화합니다.')
html = html.replace('2. 검은 윤곽선', '2. 빛과 그림자')
html = html.replace('모든 화면의 구조감을 outline과 rounded pill로 통일합니다.', '미세한 반투명 테두리와 발광하는 오라 그라데이션이 구조를 신비롭게 구분합니다.')
html = html.replace('엄청 쉬워. 그냥 했어. 이쁘기도 하고.', '마치 예술 작품 속에 들어온 듯한 압도적인 분위기. 우아하고 감각적인 경험.')
html = html.replace('Big Promise', 'Immersive Aura')
html = html.replace('Gumroad처럼 헤드라인이 제품의 자신감을 대신해야 합니다.', '빛이 번지는 듯한 효과와 압도적인 시각적 디테일이 사용자를 매료시켜야 합니다.')
html = html.replace('Warm Commerce', 'Sensuous Depth')
html = html.replace('베이지와 크림, 검은 선, 가끔의 핑크/클레이 포인트가 종이 같은 따뜻함을 만드는지 보세요.', '깊은 블랙, 반투명한 유리 질감, 그리고 신비로운 네온 악센트가 깊이감을 형성하는지 보세요.')
html = html.replace('Rounded Control', 'Sleek & Fluid')
html = html.replace('동글고 큰 버튼과 스티커 같은 배지가 고객의 긴장을 없애는지, 동시에 값싸 보이지 않는지 확인하세요.', '매끄럽고 정교한 버튼과 흐르는 듯한 애니메이션이 세련미를 더하는지 확인하세요.')
html = html.replace('Same Bones', 'Unified Elegance')
html = html.replace('어떤 안이 Gumroad에 가장 가깝고', '어떤 안이 가장 감각적(Sensuous)이고')

html = html.replace('Hard Frame', 'Aura Glass')
html = html.replace('이 버전은 그 차이를 비교하기 위한 정사각형 기준안입니다.', '가장 감각적인 글래스모피즘과 세련된 다이내믹 그라데이션을 적용한 최종 프리미엄 안입니다.')

with open(html_path, 'w', encoding='utf-8') as f:
    f.write(html)

print("Updated HTML")

with open(md_path, 'r', encoding='utf-8') as f:
    md = f.read()

# Update terms in MD
md = md.replace('Gumroad-Style Directions', 'Sensuous Aura Directions')
md = md.replace('Gumroad-esque', 'Sensuous Aura')
md = md.replace('Gumroad', 'Sensuous Cinematic')
md = md.replace('warm paper-like surfaces', 'deep cinematic backgrounds with glassmorphism')
md = md.replace('black outlines', 'soft glowing auras and fine 1px translucent borders')
md = md.replace('Hard Frame', 'Aura Glass')
md = md.replace('rounded pills', 'sleek, fluid controls')
md = md.replace('beige, cream, ink', 'deep dark tones, translucent glass, and vibrant aura accents')
md = md.replace('warm beige', 'deep dark tones')
md = md.replace('black outline', 'translucent glowing outline')
md = md.replace('square corners', 'smooth fluid corners')

with open(md_path, 'w', encoding='utf-8') as f:
    f.write(md)

print("Updated MD")
