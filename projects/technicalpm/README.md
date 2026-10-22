# Technical Project Management in Living and Geometric Order (한국어 번역)

위스콘신 대학교 매디슨 캠퍼스가 Pressbooks로 펴낸 개방형 교재
[*Technical Project Management in Living and Geometric Order*](https://wisc.pb.unizin.org/technicalpm/)
(3판, Jeffrey S. Russell, Wayne P. Pferdehirt, John S. Nelson)를 한국어로 옮긴
비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
작업하였습니다.

## 범위

- 원문: Pressbooks가 내보낸 EPUB 2.0의 압축을 푼 트리 `upstream/`. 공개 소스
  저장소가 없어 서브모듈 대신 트리를 커밋하고, 출처 URL·내려받은 날짜·원본
  EPUB의 SHA-256을 `upstream.lock`에 적습니다.
- 번역 대상: `upstream/OEBPS/*.html`(XHTML 1.1) 가운데 이미지뿐인 표지와 빌드가
  다시 만드는 목차를 뺀 28개 문서. 본문·제목·캡션·학습 상자·각주, 이미지 `alt`와
  요소 `title` 속성, 각 문서의 `<title>`을 번역합니다.
- 목차(`table-of-contents.html`)와 NCX 목차 레이블, OPF 책 제목은 빌드가 번역된
  장·절 제목에서 그대로 옮깁니다.
- 그림 속 글자, URL, 사람 이름, 참고 문헌의 서지 사항은 원문을 유지합니다. 널리
  알려진 기업명과 방법론 이름은 한국어 관용 표기를 따릅니다(`terms.toml`).
- 책의 중심 개념 *living order*/*geometric order*는 "살아 있는 질서"/"기하학적
  질서"로 옮기고 프로젝트 관리 용어는 PMBOK 지침서 한국어판의 관례를 따릅니다
  (`glossary.toml`).

## 라이선스

원서는 Board of Regents of the University of Wisconsin System의 저작물로
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)이며 라이선스 문구는
`upstream/OEBPS/copyright.html`에 있습니다. 이 번역은 원서를 개작한 2차적
저작물로서 같은 CC BY 4.0으로 제공하고, 판권 페이지와 웹 판의 모든 페이지 하단에
저작자, 원문 링크, 라이선스, 번역·수정 사실을 표시합니다.

- 부록 B의 PDCA 그림(Christoph Roser, AllAboutLean.com)은 CC BY-SA이며 수정 없이
  출처 표기와 함께 싣습니다.
- 1장의 그림 한 점(The Minerals, Metals & Materials Society)은 원서에 "Used with
  permission"으로 실린 것이라 그 허락이 번역본에 미치지 않습니다. 빌드가 이미지를
  빼고 원서를 가리키는 자리 표시로 바꿉니다.

저자의 공식 번역이 아닙니다.

## 재현

```sh
cd projects/technicalpm
../../target/release/yeokja translate upstream/OEBPS        # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/OEBPS
nix develop path:../../nix#technicalpm -c ../../target/release/yeokja build html   # dist/site (EPUB 포함)
nix develop path:../../nix#technicalpm -c ../../target/release/yeokja build epub   # dist/technicalpm-ko.epub
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의
원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와 원문에서 언제든
재생성되므로 커밋하지 않습니다.

빌드(`scripts/build_epub.py`)는 조립 트리를 `build/tree/_epub`에 실제 파일로
옮긴 뒤 목차·NCX·OPF를 한국어로 맞추고, 판권 페이지에 번역 고지를 넣고, 1장의
그림을 빼고, `mimetype`을 무압축 첫 항목으로 두어 EPUB을 묶습니다.
`scripts/build_site.py`는 같은 책을 spine 순서의 웹 페이지로 내놓고 이전·다음
링크와 공통 하단, 랜딩 페이지를 붙입니다.

검증:

```sh
python3 scripts/check.py skeleton upstream/OEBPS ko/OEBPS            # 번역문의 요소 구조가 원문과 같은지
python3 scripts/check.py epub upstream build/tree/technicalpm-ko.epub  # 목차·NCX가 본문 제목과 같은지, 언어가 ko인지
epubcheck build/tree/technicalpm-ko.epub                           # 원문 EPUB과 같은 RSC-006·RSC-012 각 3건 외에 없어야 함
```

원문 EPUB의 `epubcheck` 결과는 원격 리소스 참조(RSC-006)와 없는 조각 링크(RSC-012)
각 3건이며 번역본도 같은 것만 남아야 합니다. Deploy Pages 워크플로는 커밋된 번역
상태로 `ko/`를 재구성하고 미번역 여부를 검사한 뒤 빌드해, `dist-technicalpm`
산출물을 Pages의 `technicalpm/`에 배치합니다.
