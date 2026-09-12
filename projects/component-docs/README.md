# WebAssembly 컴포넌트 모델 한국어 번역

[Bytecode Alliance의 component-docs](https://github.com/bytecodealliance/component-docs)
중 `component-model/src/**/*.md` 전체(목차와 포함 문서 포함)를 [Yeokja](https://github.com/yeokja/yeokja)로 번역합니다.
예제 프로그램, 명령어, WIT 정의와 API 식별자는 원문을 보존합니다.

- Provider: `claude_code`
- Model: `claude-sonnet-5` (Anthropic 사의 `claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다)
- 번역 상태: `state/` (출력 `ko/`는 상태에서 재구성)
- 빌드: mdBook 0.5.3, mdbook-tabs 1.0.1
- 배포 경로: `component-docs/`

저장소 루트에서 실행합니다.

```sh
git submodule update --init projects/component-docs/upstream
target/release/yeokja -C projects/component-docs translate upstream/component-model/src
target/release/yeokja -C projects/component-docs status --check upstream/component-model/src
target/release/yeokja -C projects/component-docs build html
```

GitHub Pages는 커밋된 상태의 완역 여부를 먼저 검사한 뒤 한국어 소스를 재구성합니다.
원문과 번역본 HTML의 제목 구조를 대조하여 원문 앵커를 추가하므로 영어 앵커를
사용하는 문서 링크도 유지됩니다. 언어별 탭은 upstream 전처리기를 사용합니다.
외부 웹사이트 가용성에 영향을 받는 linkcheck2 출력과 upstream 분석 스크립트는
배포용 복사본에서 제외합니다.

원문 © 2023 The Bytecode Alliance Contributors,
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
이 한국어 번역은 원문을 번역·수정한 문서이며 같은 CC BY 4.0으로 제공합니다.
원문 라이선스는 upstream/LICENSE.md와 배포물 LICENSE.md에 포함됩니다.

빌드는 일반 문서와 인쇄용 합본의 내부 링크를 원문과 비교하여 새로 깨진 링크가
있으면 실패합니다. 원문 자체에 존재하는 잘못된 링크는 별도로 구분합니다.

## 로컬 검증 결과

- Markdown 50개, 세그먼트 2,144개 번역 완료. 미번역·변경된 원문 0개.
- CI 재구성 시 번역 세그먼트 0개로 완료(추가 provider 호출 없음).
- mdBook HTML 빌드, 원문 앵커 보존, 인쇄용 합본 빌드 성공.
- 원문과 같은 방식으로 합본을 구성하여 비교한 새 내부 링크 오류 0개.
- Pages 스테이징에서 실제 산출물과 지문 복사 확인.
- 배포 스크립트 기존 테스트 32개 통과.

자동 평가 경고 8개는 일반 의미의 world/future, 고유명사와 코드 안의 용어,
.NET으로 시작하는 Markdown 문장에 관한 오탐으로 본문과 대조했습니다.
브라우저 연결을 사용할 수 없어 화면의 시각적 검증은 수행하지 못했습니다.
GitHub 배포 완료 여부는 별도로 Actions와 공개 URL에서 확인해야 합니다.
