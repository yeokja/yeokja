# Zero to Nix 한국어 번역

[DeterminateSystems/zero-to-nix](https://github.com/DeterminateSystems/zero-to-nix)의
MDX 문서를 yeokja로 한국어 번역합니다. Nix 설치부터 개발 환경, 패키지 빌드까지
따라가는 빠른 시작과 Nix의 핵심 개념을 설명하는 개념 문서로 이루어진 Astro
사이트입니다. 원본은 `upstream` 서브모듈에 고정하며, 번역 상태는 `state/`에
커밋합니다. 사이트 chrome(내비게이션, 푸터, 안내문)은
`patches/korean-localization.patch`로 손수 한국어화하고, 이 미러가 운영하지 않는
PostHog 분석, 쿠키 배너, 뉴스레터 가입, 페이지 피드백 위젯은 같은 패치에서
제거합니다.

게시 주소: <https://yeokja.moreal.dev/zero-to-nix/>

## 번역과 빌드

저장소 루트에서 실행합니다. 번역에는 기존 프로젝트와 동일한 Claude CLI가,
HTML 빌드에는 Node.js(24 이상)와 npm이 필요합니다. 빌드 명령이 upstream의
`package-lock.json`으로 의존성을 설치하므로 별도 준비는 없습니다.

```sh
git submodule update --init projects/zero-to-nix/upstream
target/release/yeokja -C projects/zero-to-nix translate upstream/src/content
target/release/yeokja -C projects/zero-to-nix status --check upstream/src/content
target/release/yeokja -C projects/zero-to-nix build html
```

`ko/`를 원본 위에 조립하고 패치를 적용한 뒤 `astro build`로 `dist/site/`를
만듭니다. 사이트는 `/zero-to-nix/` 하위 경로에 배포되므로 Astro의 `base`
설정으로 자산 경로를, `scripts/rewrite-base.mjs`로 MDX 본문과 컴포넌트에 적힌
루트 절대 링크(`/concepts/flakes` 등)를 접두사가 붙은 경로로 고칩니다. CI는
`upstream/src/content` 전체의 번역 상태를 검사하므로 누락된 문서가 있으면
배포 전 단계에서 실패합니다.

## 라이선스와 범위

원문은 Determinate Systems, Inc.의 저작물로
[CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.ko)에
따라 배포됩니다. 이 번역본은 같은 라이선스를 따르는 비상업적 2차 저작물이며,
Determinate Systems의 공식 번역이 아닌 기계 번역입니다. 사이트 곳곳에 원문
주소와 라이선스를 밝히는 안내를 넣었습니다. 코드 블록, 명령어, 설치 스크립트는
번역 대상에 넣지 않으며 `upstream/LICENSE`의 고지를 그대로 유지합니다.
