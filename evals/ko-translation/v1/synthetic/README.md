# 합성 eval 항목

한국어 번역 eval([설계 문서](../../README.md) 4.2절)의 합성 항목을 만드는 작은
yeokja 프로젝트입니다. 과거 번역에서 실제로 깨졌던 유형(참조 링크, GFM 경고
표시, 인라인 수식, 긴 문단, 용어집과 코드 식별자의 충돌, 번역투를 부르는 영어
문장, 촘촘한 인라인 마크업, reStructuredText 역할과 각주, LaTeX 명령과 각주)을
짧은 문단으로 재현합니다.

- `src/md/` Markdown 6개(`inline_tags = true`), `src/rst/` reStructuredText 2개,
  `src/tex/` LaTeX 2개
- `glossary.toml` 용어집. 일부 용어는 본문의 코드 식별자(`borrow()`,
  `resolver`, `workspace`, `cache`, `build_script`)와 일부러 겹칩니다.
- `probes.toml` 블록별 시험 대상(`probes`)과 무엇이 잘못될 수 있는지(`note`)

## 라이선스

`src/`의 영어 원문과 이 디렉터리의 모든 파일은 이 저장소를 위해 직접 쓴
글이며 `MIT OR Apache-2.0`으로 배포합니다. 다른 문서에서 옮겨 온 문장은 없습니다.
본문의 `example.org` 주소와 문헌 키(`crosby2003`, `kildall1973`)는 자리표시자입니다.

## 쓰는 법

실제로 번역하지 않습니다. `yeokja-eval`이 이 디렉터리를 프로젝트로 읽어 실제
항목과 같은 방식(`yeokja.toml`의 파서와 `batch_segments` 묶음 규칙)으로 요청을
만들고 `v1/synthetic.jsonl`에 동결합니다. `[provider]`는 자리표시자이며 실행할
때 후보 모델로 바뀝니다. 커스텀 `prompt_template`을 두지 않으므로 운영 기본
프롬프트(GFM 경고 마커 보존 규칙 포함)가 그대로 쓰입니다.

`probes.toml`의 `block`은 파서가 매기는 블록 ID(`section:N/block:M`)이고, 그
블록의 세그먼트는 `<block>/seg:K`입니다. 추출기는 이 표를 읽어 각 블록의
`strata`에 probe를 넣고, 보고서는 probe별로 자동 관문 통과율과 쌍대 판정을 나눠
집계합니다. 모든 합성 블록은 판정 표본에 들어갑니다.

파싱 결과는 다음 명령으로 확인합니다. 둘 다 상태 파일을 쓰지 않습니다.

```sh
cargo build -p yeokja-cli
cd evals/ko-translation/v1/synthetic
../../../../target/debug/yeokja status .     # 10 files, 132 segments
../../../../target/debug/yeokja coverage .
```

`state/`, `ko/`, 빌드 산출물은 `.gitignore`로 막아 두었습니다. 원문을 고치면
블록 ID가 바뀌므로 `v1`은 고치지 않고 `v2`를 새로 만듭니다.

## 파서 동작에서 알아 둘 점

- Markdown 파서는 수식 확장을 켜지 않으므로 `$...$`는 일반 텍스트로 세그먼트에
  들어가고 인라인 태그로 보호되지 않습니다.
- `> [!NOTE]` 본문에 빈 `>` 줄이 있으면 문단마다 다른 블록이 됩니다
  (`alerts.md`의 block:5, block:6). 마커는 어느 세그먼트에도 들어가지 않습니다.
- LaTeX 파서는 블록을 문장 단위로 나누지 않아 블록마다 세그먼트가 하나이고,
  `\footnote{...}`는 본문 세그먼트 안에 통째로 들어갑니다.
- LaTeX 파서는 `\emph`, `\textbf`, 수식 안 `\text`의 인자를 마지막 섹션에 보조
  블록(`latex-aux`)으로 따로 내놓습니다. 수식 안 `\text{nodes}`도 번역 대상이
  됩니다.
