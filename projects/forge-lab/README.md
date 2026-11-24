# FORGE Lab 블로그 한국어 번역

[FORGE Lab 블로그](https://forge-blog.github.io)([microsoft/forge-lab](https://github.com/microsoft/forge-lab)의
`blog/`) — Microsoft Security의 Frontier Offensive Research & Generative Exploitation(FORGE) Lab이 쓴 연구
노트 — 를 영어에서 한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

이 번역은 원문을 수정한 개작물이며, Microsoft가 만들거나 후원하거나 검토하지 않았습니다. 번역 사이트의 모든
쪽에 같은 고지와 해당 원문 쪽 링크가 있습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `48f83fcbe88c164e7a075aec301dc22b18e907e2`(`main`)
- 번역 대상: `upstream/blog/content/posts/**/*.md`의 글 2개(front matter의 제목·설명, 본문, 표, 알림 상자, 그림 설명)
- 제외:
  - **개인정보 처리 방침 쪽(`blog/content/privacy`).** 원문 사이트의 Azure Application Insights 방문 통계를
    설명하는 쪽인데, 번역 사이트는 방문 통계를 싣지 않습니다. 쪽과 하단 링크를 함께 뺍니다.
  - **SVG 도표 속 글자.** 영어로 둡니다. 도표마다 그림 설명(alt)이 내용을 모두 풀어 쓰며, 그 설명은 번역합니다.
  - **발표 자료(`slides/`)와 CVE 목록(`cve/cve.json`).** 블로그 글이 아닙니다.
- 코드, 인라인 코드, CVE 식별자, 이슈·PR 번호, 사람 이름, 회사·제품·모델 이름(codename MDASH, Hyenas 등),
  인용한 이슈·패치 제목은 원문 그대로입니다. 용어집은 `glossary.toml`, 프로젝트 용어 기준은 `terms.toml`에 있습니다.

## 라이선스와 상표

- 블로그 글은 [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)(`upstream/LICENSE`, © Microsoft
  Corporation), 코드와 스크립트는 MIT(`upstream/LICENSE-CODE`)입니다. 번역은 같은 CC BY 4.0으로 제공하며, 사이트
  하단에 원저작 표시·라이선스·개작 사실을, 사이트 뿌리에 두 라이선스 파일을 둡니다. 번역 사이트용 템플릿
  사본(`overlay/layouts/`)은 원문 템플릿(MIT)을 고친 것입니다.
- 원문 README는 수정본이 Microsoft 상표·로고로 혼동을 일으키거나 Microsoft의 후원을 암시하지 않도록 요구합니다.
  그래서 Microsoft 로고와 본문에 로고를 붙이는 스크립트를 싣지 않고, 연구실 이름 "FORGE Lab"만 원문을 가리키는
  이름으로 두며, 모든 쪽에 비공식 번역 고지를 둡니다.
- 첫 화면·글 머리의 장식 그림, 공유 미리보기 그림, 파비콘은 싣지 않습니다. 원문의 이용 허락은 글(CC BY 4.0)과
  코드(MIT)만 다루고 그림은 어느 쪽에도 들지 않습니다. 글의 일부인 SVG 도표는 싣습니다.
- 원문 웹 글꼴(Literata, Inter, JetBrains Mono)은 SIL OFL이며 라이선스 파일과 함께 그대로 싣습니다.

## 재현

```sh
git submodule update --init projects/forge-lab/upstream
cd projects/forge-lab
../../target/release/yeokja translate upstream/blog/content/posts      # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/blog/content/posts
../../target/release/yeokja evaluate --mechanical-only upstream/blog/content/posts
nix develop path:../../nix#forge-lab -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`,
`dist/`는 `state/`와 원문에서 언제든 재생성되므로 커밋하지 않습니다. CI(`.github/scripts/rebuild-translations.sh
forge-lab`)는 미번역 세그먼트를 먼저 검사한 뒤 state에서 `ko/`를 재구성하므로 번역 provider를 부르지 않습니다.

## 구성

- 파서: `hugo`(인라인 태그) — TOML front matter의 `title`·`description`(`front_matter_keys`)만 번역하고 Hugo
  shortcode 줄(`{{< note >}}`)은 원문 그대로 둡니다. 번역 규칙은 `yeokja.toml`의 `prompt_template`에 있습니다.
- 빌드 툴체인: `nix/projects/forge-lab.nix`(nixpkgs Hugo extended, python3).
- `overlay/hugo.toml`: `https://yeokja.moreal.dev/forge-lab/`, `ko-kr`, `hasCJKLanguage`, 방문 통계 끔.
- `overlay/layouts/`: 원문 템플릿 사본을 한국어로 옮기고 로고·그림·방문 통계를 뺀 것(날짜 `2006년 1월 2일` 형식,
  번역 고지, 원저작 표시가 든 하단). `overlay/assets/css/translation.css`가 글꼴에 한글 대체 글꼴을 더합니다.
- `scripts/prepare_site.py`: 개인정보 처리 방침 쪽·로고·방문 통계·장식 그림 삭제, 번역 누락 검사.
- `scripts/check_upstream.py`: overlay 사본의 원본 템플릿·설정·CSS가 바뀌었거나 원문에 새 템플릿이 생기면
  실패합니다. 원문 서브모듈을 올린 뒤 바뀐 내용을 overlay에 옮기고 해시를 갱신합니다.
- `scripts/check_site.py`: 같은 Hugo·같은 baseURL로 빌드한 영어 원문과 쪽 목록, 원문에 없던 깨진 로컬
  링크·앵커, 쪽별 본문 제목·그림·코드 블록·표·목록·인용·알림 상자·목차 개수, 그림 설명 번역, 방문 통계·로고
  잔재, 번역 고지를 비교합니다.
