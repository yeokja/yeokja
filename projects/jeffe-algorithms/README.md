# 알고리즘 (Jeff Erickson, *Algorithms* 한국어 번역)

Jeff Erickson의 교재 [*Algorithms*](http://algorithms.wtf)(1st edition, 2019)를
한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께
Anthropic 사의 `claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두
비허용한 상태로 작업하였습니다. 저자가 검토하거나 승인한 번역이 아닙니다.

## 원서와 영어 원고

저자는 LaTeX 원고를 공개하지 않았습니다. 이 프로젝트의 `source/`는 원서 PDF에서
**복원한 영어 원고**이며 저자의 원고가 아닙니다.

- 원서 PDF: `upstream/Algorithms-JeffE.pdf`
  - 서브모듈 커밋 `9d4f235ac54e094594e7db6332e20489ba8e830b` (2019-07-01)
  - SHA-256 `00295b82ff725c52a71351a0bb1ee1bdb1584e594a62395d3fb382aefd0f4d42`, 472쪽
- 복원 방법: 쪽마다 이미지와 글꼴로 분류한 텍스트를 추출하고(`scripts/extract.py`),
  벡터 그림 203개와 래스터 이미지 9개를 잘라내고(`scripts/extract_figures.py`),
  그 텍스트를 옮겨 LaTeX 구조를 다시 세웠습니다. 본문 문장은 사람이나 모델이
  다시 타이핑하지 않고 추출 결과에서 옮겼습니다(원서의 오탈자까지 보존하기
  위해서입니다). 복원 작업은 Anthropic 사의 `claude-opus-5` 모델이 수행했습니다.
- 검증: 복원 원고를 다시 컴파일해 원서 텍스트 레이어와 단어 단위로 대조했습니다
  (`scripts/verify_source.py`). 16개 단위 모두 불일치 0.50% 이하이고, 그림·각주·
  연습문제·절 제목·의사코드 절차 개수가 원서와 일치합니다. 규칙과 검증 절차는
  `RECONSTRUCTION.md`에 있습니다.

## 범위

- 포함: 서문, 0~12장(연습문제·각주 포함), Image Credits, Colophon
- 제외
  - 색인 3종(Index, Index of People, Index of Pseudocode): 쪽 번호가 달라지므로
    후속 작업으로 둡니다.
  - 강의 노트(Extended Dance Remix, Director's Cut, Models of Computation):
    CC BY-NC-SA 4.0으로 라이선스가 다릅니다.
  - 그림 1.25: 작가의 허락으로만 수록된 초상화라 CC BY 범위 밖입니다.
- 그림은 원서 PDF에서 잘라낸 것이며, 그림 안의 영어 문구는 번역하지 않았습니다.
- 의사코드는 원서와 일본어판의 관례대로 키워드와 절차 이름을 영어로 두고 주석만
  번역했습니다.

## 라이선스

원서 © 2019 Jeff Erickson, [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
복원한 영어 원고와 한국어 번역본은 원서를 수정한 개작물로서 같은 CC BY 4.0으로
제공합니다. 비저자 그림의 출처와 라이선스는 번역본의 Image Credits에 원서대로
적혀 있고, 한국어판 판권 면(`source/frontmatter/translation-notice.tex`)에 번역·
복원 사실과 제외 항목을 밝혔습니다.

## 재현

```sh
cd projects/jeffe-algorithms
ND="nix develop path:../../nix#jeffe-algorithms -c"

# 원서에서 쪽 자료와 그림 추출 (build/extract, source/figures)
$ND python3 scripts/extract_figures.py upstream/Algorithms-JeffE.pdf source/figures build/extract/figures.json
$ND python3 scripts/extract.py upstream/Algorithms-JeffE.pdf build/extract build/extract/figures.json

# 복원 원고 검증 (장 하나씩, 예: 2장)
$ND python3 scripts/verify_source.py 02

# 번역과 한국어 PDF
../../target/release/yeokja translate source
../../target/release/yeokja status --check source
$ND ../../target/release/yeokja build pdf        # output/pdf/Algorithms-ko.pdf
```

`state/`와 `source/`는 커밋합니다. `ko/`, `build/`, `output/`은 언제든 재생성할 수
있으므로 커밋하지 않습니다.

## 도구

| 스크립트 | 하는 일 |
|---|---|
| `extract.py` | 쪽 이미지(150dpi)와 스팬별 텍스트·글꼴·분류·링크를 뽑습니다 |
| `extract_figures.py` | 벡터 그림을 독립 PDF로, 래스터를 원본 해상도로 잘라냅니다 |
| `draft_tex.py` | 추출 결과에서 LaTeX 초안을 기계적으로 만듭니다 |
| `pageview.py` | 쪽을 글꼴 표시가 붙은 줄 단위로 보여 줍니다 |
| `verify_source.py` | 복원본을 컴파일해 원서와 단어·구조·수식을 대조합니다 |
| `sidebyside.py` | 원서와 복원본 쪽을 나란히 놓은 이미지를 만듭니다 |
| `prepare_pdf.py` | 한국어 빌드 전에 조립 트리를 점검합니다 |
