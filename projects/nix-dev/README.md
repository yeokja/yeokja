# nix.dev 한국어 번역

[NixOS/nix.dev](https://github.com/NixOS/nix.dev)의 MyST Markdown 문서를
yeokja로 한국어 번역합니다. nix.dev는 Nix 공식 문서 포털로, 설치와 첫걸음부터
NixOS 배포까지 따라가는 튜토리얼, 상황별 가이드와 레시피, 핵심 개념 설명, 각
버전의 Nix 참조 매뉴얼 안내로 이루어진 Sphinx 사이트입니다. 원본은 `upstream`
서브모듈에 고정하며, 번역 상태는 `state/`에 커밋합니다. 사이트 chrome(설정,
사이드바 템플릿, robots.txt)은 `overrides/`로 덮어 한국어화합니다.

게시 주소: <https://yeokja.moreal.dev/nix-dev/>

## 번역과 빌드

저장소 루트에서 실행합니다. 번역에는 기존 프로젝트와 동일한 Claude CLI가,
HTML 빌드에는 Python 3(3.12 이상, 3.14에서 확인)과 Pagefind 검색 색인을 위한
Node.js(`npx`)가 필요합니다. 빌드 명령이 `requirements.txt`의 Sphinx
툴체인을 `build/venv`에 설치하므로(파일이 바뀌지 않으면 재사용) 별도 준비는
없습니다. 다른 Python을 쓰려면 `NIX_DEV_PYTHON=python3.12`처럼 지정합니다.
GitHub Actions에서는 워크플로가 같은 requirements를 Python 3.12에 미리
설치합니다.

```sh
git submodule update --init projects/nix-dev/upstream
target/release/yeokja -C projects/nix-dev translate upstream/source
target/release/yeokja -C projects/nix-dev status --check upstream/source
target/release/yeokja -C projects/nix-dev build html
python3 -m unittest discover -s projects/nix-dev/scripts -p 'test_*.py'
```

`ko/`와 `overrides/`를 원본 위에 조립하고 `scripts/build-site.sh`가
`dist/site/`를 만듭니다. 절차와 설계는 다음과 같습니다.

- **설정 덮어쓰기**: `overrides/source/conf.py`가 upstream의 `conf.py`를 그대로
  실행한 뒤 `language = "ko"`, 기준 URL, 비공식 번역 안내 배너(sphinx-book-theme
  `announcement`), 404 접두사를 덧붙입니다. `-D` 옵션으로 확장 목록을 통째로
  다시 적거나 `conf.py`에 패치를 대는 대신 이 방식을 택해, upstream이 확장이나
  테마 옵션을 바꿔도 따라갈 것이 없습니다. 저장소 버튼은 upstream을 가리키고,
  `contributors` 확장과 `_templates/search.html`(Pagefind UI)은 그대로 씁니다.
- **영어 앵커 유지**: nix.dev는 제목 텍스트에서 만든 슬러그(`#some-heading`)로
  문서끼리 링크합니다. `scripts/ko_slugs.py` 확장이 원문과 번역본의 제목을
  자리별로 짝지어 번역된 제목에도 원문 슬러그를 부여하고, 그 슬러그를 섹션의
  HTML id로 올립니다. 그래서 문서 간 링크, 페이지 안 목차, 외부에서 들어오는
  깊은 링크가 모두 원문과 같은 앵커를 씁니다. 제목 수가 원문과 다른 문서는
  경고를 내고 기본 슬러그로 돌아갑니다.
- **버전 자리 표시자**: `source/reference/nix-manual.md`의 `@nix-latest@` 등은
  upstream이 Nix로 계산해 치환합니다. `scripts/nix-releases.py`가
  `upstream/nix/*.json`의 고정본에서 같은 값을 구합니다. Nixpkgs 각 릴리스에
  들어 있는 Nix 버전은 Nixpkgs를 평가해야 나오므로, 고정된 리비전의
  `pkgs/tools/package-management/nix/default.nix`를 GitHub에서 한 번 읽어
  `nixpkgs-nix-versions.json`에 기록해 둡니다. upstream이 고정본을 올리면
  로컬 빌드가 새 리비전을 받아 이 파일을 갱신하므로 함께 커밋하세요.
- **실파일 source/**: 조립 트리는 심링크 묶음이라 myst가 문서 링크를 실제
  경로로 풀면 srcdir 바깥이 되어 링크가 깨집니다. 빌드 스크립트가 `source/`만
  관통 복사해 실파일로 만든 뒤 Sphinx를 돌립니다.
- **검색**: `npx -y pagefind@1`로 색인을 만들어 `site/pagefind/`에 둡니다.
  색인 생성이 실패하면 빌드도 실패합니다.

Nix 참조 매뉴얼(`/manual/nix/...`)은 nix.dev가 따로 호스팅하는 별도 문서로 이
미러의 범위가 아니며, 본문의 링크는 모두 `https://nix.dev/manual/...` 절대
주소라 그대로 원문 사이트로 연결됩니다. 사이드바의 PDF 링크도 원문 PDF를
가리킵니다. CI는 `upstream/source` 전체의 번역 상태를 검사하므로 누락된 문서가
있으면 배포 전 단계에서 실패합니다.

## 라이선스와 범위

원문은 NixOS Foundation과 기여자의 저작물로
[CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.ko)에 따라
배포됩니다(`upstream/LICENSE.md`). 이 번역본은 동일조건변경허락 조항에 따라
같은 라이선스로 배포하는 2차 저작물이며, Nix 문서 팀의 공식 번역이 아닌 기계
번역입니다. 사이트 상단 배너와 사이드바에 원문 주소와 라이선스를 밝힙니다.
코드 블록, 명령어, Nix 표현식과 지시문 옵션은 번역 대상에 넣지 않습니다.
