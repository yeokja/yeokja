# HoTT Book 한국어 번역

[HoTT/book](https://github.com/HoTT/book)의 비공식 한국어 번역입니다.
원저작자는 The Univalent Foundations Program입니다. 원문과 번역은
[CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/)으로 배포합니다.
이 프로젝트는 원저작자가 번역을 검토하거나 보증한다는 뜻이 아닙니다.

## 번역과 재현

원문은 `upstream/` 서브모듈에 고정합니다. 서문, 본문 전체, 부록, 기호 설명,
표지와 뒤표지의 19개 원고 및 별도로 추출한 찾아보기 용어를 yeokja의 `latex-extended` 파서로 처리합니다. 번역 provider는
`codex`, 모델은 `gpt-6-astra`, 추론 강도는 `medium`입니다.
`glossary.toml`은 수학 용어를 통일합니다. 번역은 합쇼체로 작성하며, 수식·라벨·
참고문헌 키·원저자 이름을 보존합니다. 상태 파일 `state/`를 커밋하고, `ko/`는
상태에서 재구성합니다. 레이아웃의 고정 문구와 한글 글꼴은 별도 패치로 적용합니다.

저장소 루트에서 최신 실행 파일을 빌드한 뒤 실행합니다.

```sh
cargo build --release -p yeokja-cli
git submodule update --init projects/hott/upstream
cd projects/hott
python3 scripts/index_terms.py extract
../../target/release/yeokja translate index
../../target/release/yeokja translate upstream
../../target/release/yeokja status index --check
../../target/release/yeokja status upstream --check
../../target/release/yeokja evaluate upstream --mechanical-only
../../target/release/yeokja coverage upstream --min-lines 3
../../target/release/yeokja build pdf
```

번역 시 Codex CLI의 ChatGPT 로그인이 필요합니다. 이미 번역된 state만으로
출력을 재구성하고 PDF를 빌드할 때는 모델 호출이 필요하지 않습니다.

## PDF와 GitHub Pages

Ubuntu 24.04의 다음 패키지로 빌드합니다.

```sh
sudo apt-get install fonts-nanum fonts-texgyre latexmk texlive-xetex texlive-latex-extra \
  texlive-fonts-recommended texlive-science texlive-lang-korean
```

`output/pdf/HoTT-ko.pdf`가 배포 결과물입니다. GitHub Pages 워크플로의
`latex-hott` 작업이 같은 툴체인으로 PDF를 빌드하고 `/hott/HoTT-ko.pdf`에
배치합니다. 배포 전 19개 원고 전체의 번역 상태를 확인하고, 미번역이 있으면
실패합니다. 글자 누락이나 해결되지 않은 참조가 PDF 로그에 있어도 빌드를 중단합니다. 빌드 입력이 같으면 이전 PDF를 보존합니다.

원문에 포함된 별도 `exercise_solutions.tex`, `errata.tex`, `coq_introduction/`은
`hott-online.tex` 책에 포함되지 않는 보조 자료로, 이 책 번역의 대상이 아닙니다.

찾아보기의 표시 용어는 `index/terms.tex`에 한 번씩 추출하여 같은 provider로
번역합니다. 빌드 시 원문의 정렬 키와 범위 표식을 유지하고 모든 표시 용어를
동일한 한국어 사전으로 바꿉니다. 본문에서 이동한 표식은 번역 위치를 유지하며,
표식 개수가 맞지 않으면 PDF 빌드가 실패합니다. 인용한 문헌 제목은 원문을 유지합니다.

`latex-extended`는 HoTT의 정리 약어, 목록 제목, 수식 설명을 추가로 추출하며, 여러 줄의 자체 수식 매크로와 추론 규칙을 보존합니다.
기존 `latex` 프로젝트의 세그먼트 배치와 번역 상태는 바꾸지 않습니다.

찾아보기 표식이 문장 중간에 있더라도 문장은 하나로 번역하며, 표식의 원문 키는
조판 시 복원한 뒤 공통 한국어 표시 사전을 적용합니다.

## 검증한 판 (2026-09-12)

원문 커밋은 `578b85cc8d586b1677ec4335148adeb443057d24`입니다.
본문 6,057개와 찾아보기 1,291개 세그먼트의 번역 상태가 100%이며,
전체 기계적 검사에서 오류가 없음을 확인했습니다. 생성 후 수식 공백 한 곳과
여분의 닫는 괄호 두 곳을 원문과 대조해 교정했습니다.
한국어 PDF는 468쪽이며, 글자 누락·미해결 참조 검사와 표지·목차·본문·찾아보기의
렌더링 검사를 거칩니다. 배포용 재구성은 Codex 실행을 차단한 상태에서도 동작합니다.
