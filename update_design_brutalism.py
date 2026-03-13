import os
import re

html_path = r'c:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-directions.html'
md_path = r'c:\Code\Project\Boothy\_bmad-output\planning-artifacts\ux-design-specification.md'

with open(html_path, 'r', encoding='utf-8') as f:
    html = f.read()

# Replace Root CSS
html = re.sub(r':root\s*\{[^}]+\}', """:root {
      --bg: #ece5df;
      --surface: #ffffff;
      --surface-2: #f0f0f0;
      --panel: #ffffff;
      --ink: #000000;
      --ink-soft: #444444;
      --line: #000000;
      --accent: #ff3366;
      --accent-glow: rgba(0, 0, 0, 1);
      --pink: #ff3366;
      --clay: #e48c6c;
      --butter: #fcd34d;
      --sage: #6ee7b7;
      --sky: #7dd3fc;
      --max: 1320px;
      --radius-xl: 0px;
      --radius-lg: 0px;
      --radius-md: 0px;
      --shadow-sm: 4px 4px 0 0 #000000;
      --shadow-md: 8px 8px 0 0 #000000;
      --shadow-lg: 12px 12px 0 0 #000000;
      --aura-gradient: none;
    }""", html)

html = re.sub(r'body\s*\{\s*margin[\s\S]*?body::before\s*\{[\s\S]*?\}', """body {
      margin: 0;
      font-family: "Pretendard Variable", "Pretendard", "Noto Sans KR", sans-serif;
      color: var(--ink);
      background-color: var(--bg);
      background-image: 
        linear-gradient(rgba(0,0,0,0.1) 1px, transparent 1px),
        linear-gradient(90deg, rgba(0,0,0,0.1) 1px, transparent 1px);
      background-size: 40px 40px;
    }

    body::before {
      content: none;
    }""", html)

html = re.sub(r'\.site-header\s*\{[\s\S]*?\}', """.site-header {
      position: sticky;
      top: 0;
      z-index: 20;
      background: var(--bg);
      border-bottom: 3px solid var(--line);
    }""", html)

html = re.sub(r'\.brand-badge\s*\{[\s\S]*?\}', """.brand-badge {
      width: 42px;
      height: 42px;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border: 3px solid var(--line);
      background: var(--accent);
      box-shadow: var(--shadow-sm);
      font-weight: 800;
      color: #000;
    }""", html)

html = re.sub(r'\.nav-pill,[\s\S]*?\}\n', """.nav-pill,
    .tag,
    .mini-pill,
    .status-pill,
    .pick-chip,
    .compare-clear,
    .choose-button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      border: 3px solid var(--line);
      background: #fff;
      min-height: 42px;
      padding: 0 16px;
      font-size: 0.94rem;
      font-weight: 700;
      color: var(--ink);
      box-shadow: var(--shadow-sm);
      transition: transform 0.1s ease;
    }
""", html)

html = re.sub(r'\.nav-pill:hover,[\s\S]*?\}\n', """.nav-pill:hover,
    .choose-button:hover,
    .compare-clear:hover {
      background: var(--accent);
      transform: translate(-4px, -4px);
    }
""", html)

html = re.sub(r'\.nav-pill\.active\s*\{[\s\S]*?\}\n', """.nav-pill.active {
      background: #000;
      color: #fff;
      border-color: #000;
    }
""", html)

html = re.sub(r'\.header-button\s*\{[\s\S]*?\}\n', """.header-button {
      min-height: 48px;
      padding: 0 18px;
      border: 3px solid var(--line);
      background: #fff;
      color: var(--ink);
      font-weight: 800;
      box-shadow: var(--shadow-sm);
      transition: transform 0.1s ease;
    }
""", html)

html = re.sub(r'\.header-button:hover\s*\{[\s\S]*?\}\n', """.header-button:hover {
      background: var(--accent);
      transform: translate(-4px, -4px);
    }
""", html)

html = re.sub(r'\.header-button\.primary\s*\{[\s\S]*?\}\n', """.header-button.primary {
      background: #000;
      color: #fff;
      border-color: #000;
    }
""", html)

# Update Hero 
html = re.sub(r'\.hero\s*\{[\s\S]*?\}\n', """.hero {
      border: 3px solid var(--line);
      background: #fff;
      box-shadow: var(--shadow-lg);
      display: grid;
      grid-template-columns: 1.25fr 0.75fr;
      position: relative;
    }
""", html)

html = re.sub(r'\.hero-copy\s*\{[\s\S]*?\}\n', """.hero-copy {
      padding: 48px;
      display: grid;
      gap: 24px;
      border-right: 3px solid var(--line);
      background: var(--bg);
      position: relative;
    }
""", html)

html = re.sub(r'\.hero-panel\s*\{[\s\S]*?\}\n', """.hero-panel {
      padding: 32px;
      display: grid;
      gap: 16px;
      background: #fff;
      position: relative;
    }
""", html)

# Buttons
html = re.sub(r'\.primary-button,\s*\.secondary-button\s*\{[\s\S]*?\}\n', """.primary-button,
    .secondary-button {
      min-height: 58px;
      padding: 0 28px;
      border: 3px solid #000;
      font-size: 1.15rem;
      font-weight: 800;
      transition: transform 0.1s ease;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      box-shadow: var(--shadow-md);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }
""", html)

html = re.sub(r'\.primary-button\s*\{[\s\S]*?\}', ".primary-button { background: var(--accent); color: #000; }", html)
html = re.sub(r'\.secondary-button\s*\{[\s\S]*?\}', ".secondary-button { background: #fff; color: #000; }", html)

html = re.sub(r'\.primary-button:hover,[\s\S]*?\}\n', """.primary-button:hover,
    .secondary-button:hover {
      transform: translate(-4px, -4px);
      box-shadow: var(--shadow-lg);
    }
""", html)

# Panels
html = re.sub(r'\.hero-note,\n\s*\.hero-metric,[\s\S]*?\}\n', """.hero-note,
    .hero-metric,
    .criteria-card,
    .compare-box,
    .direction,
    .screen,
    .sub-card,
    .operator-block,
    .rail-card,
    .choice-panel {
      border: 3px solid var(--line);
      background: #fff;
      box-shadow: var(--shadow-sm);
      position: relative;
      overflow: hidden;
    }
""", html)

html = re.sub(r'\.hero-metric\s*\{[\s\S]*?\}\n', ".hero-metric { background: #fff; }\n", html)

html = re.sub(r'\.compare-box\s*\{[\s\S]*?\}\n', """.compare-box {
      background: var(--accent);
      display: grid;
      gap: 14px;
      border: 3px solid #000;
    }
""", html)

# Direction Blocks
html = re.sub(r'\.direction\s*\{[\s\S]*?\}\n', """.direction {
      padding: 32px;
      background: #fff;
      scroll-margin-top: 120px;
    }
""", html)

html = re.sub(r'\.screen\s*\{[\s\S]*?\}\n', """.screen {
      overflow: hidden;
      background: #fff;
      border: 3px solid #000;
    }
""", html)

html = re.sub(r'\.screen-head\s*\{[\s\S]*?\}\n', """.screen-head {
      border-bottom: 3px solid #000;
      padding: 16px 20px;
      display: flex;
      justify-content: space-between;
      gap: 10px;
      align-items: center;
      background: var(--bg);
    }
""", html)

html = re.sub(r'\.hero-card\s*\{[\s\S]*?\}\n', """.hero-card {
      border: 3px solid #000;
      background: #fff;
      padding: 32px;
      display: grid;
      gap: 16px;
      min-height: 340px;
      align-content: end;
      position: relative;
      overflow: visible;
      box-shadow: var(--shadow-md);
    }
""", html)


html = html.replace('Sensuous Aura Directions', 'Neo-Brutalism Directions')
html = html.replace('Sensuous Aura', 'Neo-Brutalism')
html = html.replace('Sensuous redesign / Cinematic depth / Aura blur', 'Neo-Brutalism redesign / High contrast / Hard edges')
html = html.replace('Boothy를 감각적이고 압도적인 시네마틱 경험으로 재조율했습니다.', 'Boothy를 강렬하고 거친 네오브루탈리즘 스타일로 전면 재설계했습니다.')
html = html.replace('가장 직관적인 구조 위에 완전히 새로운 심미성을 얹었습니다. 깊은 어둠 속에서 발광하는 오라(Aura), 빛을 투과하는 글래스모피즘(Glassmorphism), 그리고 시네마틱한 대비가 공간 전체의 분위기를 지배합니다.', '날것 그대로의 거친 매력, 굵은 검은색 테두리, 강렬한 원색 팝 칼라, 그리고 인위적인 그림자가 특징인 네오브루탈리즘 스타일을 통해 직관적이고 기억에 남는 강렬한 경험을 선사합니다.')
html = html.replace('Cinematic mood', 'Raw expression')
html = html.replace('Glassmorphism layers', 'Flat bold colors')
html = html.replace('Glow aura & delicate lines', 'Thick dark outlines')
html = html.replace('너무 일반적이었던 기존 스타일을 버리고, 다크 모드 기반의 프리미엄 하이엔드 미학으로 완전히 재해석합니다.', '밋밋한 기존 스타일을 버리고, 과감하고 거친 네오브루탈리즘 미학으로 강렬하게 재해석합니다.')
html = html.replace('1. 세련된 타이포그래피', '1. 과감한 타이포그래피')
html = html.replace('Pretendard의 우아한 자간과 얇고 세련된 웨이트를 조합하여 고급스러움을 극대화합니다.', '크고 굵은 헤드라인 서체를 사용하여 거침없이 메시지를 전달합니다.')
html = html.replace('2. 빛과 그림자', '2. 하드 엣지 그림자')
html = html.replace('미세한 반투명 테두리와 발광하는 오라 그라데이션이 구조를 신비롭게 구분합니다.', '선명하고 진한 형태의 단색 그림자가 요소들을 물리적으로 공중에 떠 있는 듯 뚜렷하게 분리합니다.')
html = html.replace('마치 예술 작품 속에 들어온 듯한 압도적인 분위기. 우아하고 감각적인 경험.', '파격적이지만 직관적이다. 투박함이 주는 강한 개성.')
html = html.replace('Immersive Aura', 'Raw Contrast')
html = html.replace('빛이 번지는 듯한 효과와 압도적인 시각적 디테일이 사용자를 매료시켜야 합니다.', '굵은 테두리와 강렬한 색상 대비가 즉각적으로 시선을 사로잡아야 합니다.')
html = html.replace('Sensuous Depth', 'Brutal Blocks')
html = html.replace('깊은 블랙, 반투명한 유리 질감, 그리고 신비로운 네온 악센트가 깊이감을 형성하는지 확인하세요.', '모서리가 각진 박스, 진한 테두리, 생생한 단색 배경이 거친 블록처럼 쌓여 있는지 확인하세요.')
html = html.replace('Sleek & Fluid', 'Assertive Actions')
html = html.replace('매끄럽고 정교한 버튼과 흐르는 듯한 애니메이션이 세련미를 더하는지 확인하세요.', '투박하고 명확한 크기의 버튼이 망설임 없는 행동을 유도하는지 확인하세요.')
html = html.replace('Unified Elegance', 'Unapologetic Frame')
html = html.replace('가장 감각적(Sensuous)이고', '가장 강렬한(Brutal) 그리고')

html = html.replace('Aura Glass', 'Neo-Brutalism Core')
html = html.replace('가장 감각적인 글래스모피즘과 세련된 다이내믹 그라데이션을 적용한 최종 프리미엄 안입니다.', '가장 거칠고 직관적인 네오브루탈리즘의 본질을 담은 핵심 방향입니다.')

with open(html_path, 'w', encoding='utf-8') as f:
    f.write(html)

print("Updated HTML")

with open(md_path, 'r', encoding='utf-8') as f:
    md = f.read()

# Update terms in MD
md = md.replace('Sensuous Aura Directions', 'Neo-Brutalism Directions')
md = md.replace('Sensuous Aura', 'Neo-Brutalism')
md = md.replace('Sensuous Cinematic', 'Retro-Digital')
md = md.replace('deep cinematic backgrounds with glassmorphism', 'flat stark backgrounds with brutal primary colors')
md = md.replace('soft glowing auras and fine 1px translucent borders', 'thick black stark outlines and high-contrast offset shadows')
md = md.replace('Aura Glass', 'Brutal Core')
md = md.replace('sleek, fluid controls', 'heavy, oversized block controls')
md = md.replace('deep dark tones, translucent glass, and vibrant aura accents', 'solid white/beige offset by brutalist primary colors and heavy black lines')
md = md.replace('deep dark tones', 'solid colors')
md = md.replace('translucent glowing outline', 'thick dark outline')
md = md.replace('smooth fluid corners', 'sharp square corners')

with open(md_path, 'w', encoding='utf-8') as f:
    f.write(md)

print("Updated MD")
