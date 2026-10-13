# 한국어 번역 eval 보고서

- 세트: `evals/ko-translation/v1`
- 기준선: `opus-5-5`
- 채점: `evals/ko-translation/judgments/2026-10-14-surroundings`

## 1. 자동 관문

블록 단위 통과율. 첫 시도는 모델의 첫 응답(운영과 같은 기계 수선 후), 최종은 재시도 루프를 거친 결과입니다.

| 후보 | 최종 통과 | 첫 시도 통과 | 판정 표본 최종 통과 | 마크업 감사 수선 | 요청 실패 | 모델 호출 | 출력 토큰 | 소요 시간(합) |
|---|---|---|---|---|---|---|---|---|
| opus-5-5 | 99.9% (1052/1053) | 98.1% (1033/1053) | 100.0% (225/225) | 0 | 0/82 | 92 | 110649 | 22분 |
| opus-5-5-surroundings | 99.9% (1052/1053) | 98.0% (1032/1053) | 100.0% (225/225) | 0 | 0/82 | 93 | 111008 | 22분 |

실패한 검사(최종, 블록 수):

| 후보 | present | evaluators | audit | alignment | alert-markers | truncation |
|---|---|---|---|---|---|---|
| opus-5-5 | 0 | 1 | 0 | 0 | 0 | 0 |
| opus-5-5-surroundings | 0 | 1 | 0 | 0 | 0 | 0 |

## 2. 익명 쌍대 비교 (기준선 `opus-5-5` 대비 후보 승률)

두 관문을 모두 통과한 판정 표본 블록만 비교합니다. A/B 순서를 바꾼 두 판정이 같을 때만 승패로 세고, 엇갈리면 무승부입니다. 괄호 안은 윌슨 95% 신뢰 구간, 승률은 무승부를 뺀 값입니다. 판정 오류 0건.

### 종합

| 채점자 | opus-5-5-surroundings |
|---|---|
| gemini-flash | 48% [40–56] (67승 72패 86무) |
| gpt-6-astra | 55% [46–63] (77승 64패 84무) |

### 정확성

| 채점자 | opus-5-5-surroundings |
|---|---|
| gemini-flash | 39% [24–58] (11승 17패 197무) |
| gpt-6-astra | 39% [26–53] (17승 27패 181무) |

### 자연스러움

| 채점자 | opus-5-5-surroundings |
|---|---|
| gemini-flash | 49% [41–58] (62승 64패 99무) |
| gpt-6-astra | 58% [49–66] (76승 56패 93무) |

### 번역투(적을수록 승)

| 채점자 | opus-5-5-surroundings |
|---|---|
| gemini-flash | 53% [43–62] (54승 48패 123무) |
| gpt-6-astra | 56% [46–65] (56승 44패 125무) |

## 3-1. 채점자 간 일치율 (종합)

`gemini-flash`와 `gpt-6-astra`가 모두 판정한 쌍에서 종합 판정(순서를 바꾼 두 판정을 합친 것)이 같은 비율: 70.7% (159/225)

## 4. 프로젝트·파서별 종합 승률

### gemini-flash

| 프로젝트 (파서) | opus-5-5-surroundings |
|---|---|
| chisel-book (latex) | 0승 3패 1무 |
| cp-algorithms (mkdocs) | 2승 5패 8무 |
| cpython-devguide (rst) | 2승 1패 3무 |
| fp-lean (verso) | 1승 1패 0무 |
| furiosa-opt (markdown) | 2승 3패 3무 |
| hott (latex-extended) | 3승 3패 6무 |
| learn-fpga (markdown) | 1승 4패 2무 |
| mil (mil) | 1승 2패 3무 |
| nix-dev (myst) | 5승 1패 6무 |
| peps (pep) | 7승 1패 4무 |
| putting-the-you-in-cpu (mdx) | 4승 5패 3무 |
| raytracing (markdeep) | 0승 4패 4무 |
| rust-forge (markdown) | 4승 3패 7무 |
| rustc-dev-guide (markdown) | 8승 6패 6무 |
| synthetic (latex) | 4승 3패 5무 |
| synthetic (markdown) | 13승 17패 11무 |
| synthetic (rst) | 4승 3패 4무 |
| thebeambook (asciidoc) | 0승 1패 1무 |
| tpil (verso) | 1승 1패 0무 |
| webassembly-component-docs (markdown) | 2승 3패 4무 |
| zero-to-nix (mdx) | 3승 2패 5무 |

### gpt-6-astra

| 프로젝트 (파서) | opus-5-5-surroundings |
|---|---|
| chisel-book (latex) | 0승 3패 1무 |
| cp-algorithms (mkdocs) | 5승 2패 8무 |
| cpython-devguide (rst) | 2승 1패 3무 |
| fp-lean (verso) | 1승 1패 0무 |
| furiosa-opt (markdown) | 0승 5패 3무 |
| hott (latex-extended) | 3승 4패 5무 |
| learn-fpga (markdown) | 2승 4패 1무 |
| mil (mil) | 1승 3패 2무 |
| nix-dev (myst) | 4승 4패 4무 |
| peps (pep) | 7승 2패 3무 |
| putting-the-you-in-cpu (mdx) | 6승 4패 2무 |
| raytracing (markdeep) | 2승 3패 3무 |
| rust-forge (markdown) | 7승 1패 6무 |
| rustc-dev-guide (markdown) | 7승 6패 7무 |
| synthetic (latex) | 6승 1패 5무 |
| synthetic (markdown) | 14승 10패 17무 |
| synthetic (rst) | 4승 4패 3무 |
| thebeambook (asciidoc) | 1승 1패 0무 |
| tpil (verso) | 1승 1패 0무 |
| webassembly-component-docs (markdown) | 2승 2패 5무 |
| zero-to-nix (mdx) | 2승 2패 6무 |

## 5. 판정 기준 적용

사람 판정이 20쌍 미만이라, 채점자 간 일치율(70% 이상)을 신뢰도의 근거로 씁니다. 이 결론은 **LLM 채점자 근거만** 있습니다. 두 채점자가 공통으로 가진 취향은 이 방법으로 걸러지지 않습니다.

- **opus-5-5-surroundings**: 기준선을 대체할 근거가 부족함
  - 관문: 최종 99.9% (기준선 99.9%), 첫 시도 98.0% (기준선 98.1%) → 미달
  - 채점자 종합 승률 하한 > 50%: gemini-flash: 미달, gpt-6-astra: 미달
  - 사람 종합 승률 > 50%: 판정 없음
