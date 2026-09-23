# 한국어 번역 eval 보고서

- 세트: `evals/ko-translation/v1`
- 기준선: `opus-5-5`
- 채점: `evals/ko-translation/judgments/2026-09-23-opus-5-5-baseline`

## 1. 자동 관문

블록 단위 통과율. 첫 시도는 모델의 첫 응답(운영과 같은 기계 수선 후), 최종은 재시도 루프를 거친 결과입니다.

| 후보 | 최종 통과 | 첫 시도 통과 | 판정 표본 최종 통과 | 마크업 감사 수선 | 요청 실패 | 모델 호출 | 출력 토큰 | 소요 시간(합) |
|---|---|---|---|---|---|---|---|---|
| opus-5-5 | 99.9% (1052/1053) | 98.1% (1033/1053) | 100.0% (225/225) | 0 | 0/82 | 92 | 111194 | 20분 |
| fused-opus-5-5 | 99.9% (1052/1053) | 99.7% (1050/1053) | 100.0% (225/225) | 0 | 0/82 | 87 | 110783 | 20분 |

실패한 검사(최종, 블록 수):

| 후보 | present | evaluators | audit | alignment | alert-markers | truncation |
|---|---|---|---|---|---|---|
| fused-opus-5-5 | 0 | 1 | 0 | 0 | 0 | 0 |
| opus-5-5 | 0 | 1 | 0 | 0 | 0 | 0 |

## 2. 익명 쌍대 비교 (기준선 `opus-5-5` 대비 후보 승률)

두 관문을 모두 통과한 판정 표본 블록만 비교합니다. A/B 순서를 바꾼 두 판정이 같을 때만 승패로 세고, 엇갈리면 무승부입니다. 괄호 안은 윌슨 95% 신뢰 구간, 승률은 무승부를 뺀 값입니다. 판정 오류 0건.

### 종합

| 채점자 | fused-opus-5-5 |
|---|---|
| gemini-flash | 70% [58–80] (47승 20패 158무) |
| gpt-6-astra | 67% [56–76] (53승 26패 146무) |

### 정확성

| 채점자 | fused-opus-5-5 |
|---|---|
| gemini-flash | 84% [62–94] (16승 3패 206무) |
| gpt-6-astra | 74% [55–87] (20승 7패 198무) |

### 자연스러움

| 채점자 | fused-opus-5-5 |
|---|---|
| gemini-flash | 72% [59–82] (41승 16패 168무) |
| gpt-6-astra | 60% [48–71] (39승 26패 160무) |

### 번역투(적을수록 승)

| 채점자 | fused-opus-5-5 |
|---|---|
| gemini-flash | 71% [57–82] (32승 13패 180무) |
| gpt-6-astra | 52% [38–66] (23승 21패 181무) |

## 3-1. 채점자 간 일치율 (종합)

`gemini-flash`와 `gpt-6-astra`가 모두 판정한 쌍에서 종합 판정(순서를 바꾼 두 판정을 합친 것)이 같은 비율: 85.8% (193/225)

## 4. 프로젝트·파서별 종합 승률

### gemini-flash

| 프로젝트 (파서) | fused-opus-5-5 |
|---|---|
| chisel-book (latex) | 0승 0패 4무 |
| cp-algorithms (mkdocs) | 3승 2패 10무 |
| cpython-devguide (rst) | 1승 0패 5무 |
| fp-lean (verso) | 1승 0패 1무 |
| furiosa-opt (markdown) | 0승 0패 8무 |
| hott (latex-extended) | 1승 1패 10무 |
| learn-fpga (markdown) | 0승 0패 7무 |
| mil (mil) | 2승 0패 4무 |
| nix-dev (myst) | 4승 0패 8무 |
| peps (pep) | 1승 1패 10무 |
| putting-the-you-in-cpu (mdx) | 4승 1패 7무 |
| raytracing (markdeep) | 1승 1패 6무 |
| rust-forge (markdown) | 1승 1패 12무 |
| rustc-dev-guide (markdown) | 4승 4패 12무 |
| synthetic (latex) | 5승 0패 7무 |
| synthetic (markdown) | 10승 5패 26무 |
| synthetic (rst) | 5승 1패 5무 |
| thebeambook (asciidoc) | 0승 0패 2무 |
| tpil (verso) | 0승 1패 1무 |
| webassembly-component-docs (markdown) | 0승 2패 7무 |
| zero-to-nix (mdx) | 4승 0패 6무 |

### gpt-6-astra

| 프로젝트 (파서) | fused-opus-5-5 |
|---|---|
| chisel-book (latex) | 1승 0패 3무 |
| cp-algorithms (mkdocs) | 3승 1패 11무 |
| cpython-devguide (rst) | 0승 0패 6무 |
| fp-lean (verso) | 0승 1패 1무 |
| furiosa-opt (markdown) | 1승 0패 7무 |
| hott (latex-extended) | 1승 1패 10무 |
| learn-fpga (markdown) | 0승 0패 7무 |
| mil (mil) | 3승 0패 3무 |
| nix-dev (myst) | 3승 0패 9무 |
| peps (pep) | 3승 1패 8무 |
| putting-the-you-in-cpu (mdx) | 5승 3패 4무 |
| raytracing (markdeep) | 1승 1패 6무 |
| rust-forge (markdown) | 2승 4패 8무 |
| rustc-dev-guide (markdown) | 3승 4패 13무 |
| synthetic (latex) | 5승 0패 7무 |
| synthetic (markdown) | 13승 6패 22무 |
| synthetic (rst) | 6승 1패 4무 |
| thebeambook (asciidoc) | 1승 0패 1무 |
| tpil (verso) | 0승 0패 2무 |
| webassembly-component-docs (markdown) | 0승 2패 7무 |
| zero-to-nix (mdx) | 2승 1패 7무 |

## 5. 판정 기준 적용

사람 판정이 20쌍 미만이라, 채점자 간 일치율(70% 이상)을 신뢰도의 근거로 씁니다. 이 결론은 **LLM 채점자 근거만** 있습니다. 두 채점자가 공통으로 가진 취향은 이 방법으로 걸러지지 않습니다.

- **fused-opus-5-5**: LLM 채점자 기준으로 우세 (사람 검증 없음)
  - 관문: 최종 99.9% (기준선 99.9%), 첫 시도 99.7% (기준선 98.1%) → 통과
  - 채점자 종합 승률 하한 > 50%: gemini-flash: 통과, gpt-6-astra: 통과
  - 사람 종합 승률 > 50%: 판정 없음
