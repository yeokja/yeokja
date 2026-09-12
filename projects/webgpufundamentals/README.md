# WebGPU Fundamentals 한국어 번역

원본: https://github.com/webgpu/webgpufundamentals (BSD-3-Clause).
원본은 `upstream` 서브모듈 커밋으로 고정합니다. 번역은 [Yeokja](https://github.com/yeokja/yeokja)의
`claude_code` provider와 `claude-sonnet-5` 모델로 생성합니다. Anthropic 사의 `claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.
`sonnet-5` 별칭 대신 CLI에서 확인한 정식 모델 ID를 사용합니다.

영문 Markdown 66개, WGSL 함수 레퍼런스와 목차 HTML, 제목·설명·목차 및
브라우저 경고를 번역합니다. 원본의 한국어 언어 UI를 재사용하며 코드와 실행
예제를 보존합니다. 제목 메타데이터는 값만 별도로 번역하고 키와 줄 구조를
복원합니다. 원본 카메라 강의의 누락된 코드 펜스가 가린 문단도 별도로 번역합니다.

프로젝트 디렉터리에서 실행합니다:

```sh
python3 scripts/metadata.py extract
yeokja translate upstream/webgpu/lessons
yeokja translate metadata
yeokja status --check upstream/webgpu/lessons
yeokja status --check metadata
python3 scripts/metadata.py apply
python3 scripts/verify_sources.py
yeokja build html
```

필요한 툴체인(Node 24, Python 3, git, Linux에서는 Chromium)은
저장소 루트의 `nix develop path:nix#webgpufundamentals`(또는 이 디렉토리에서
`nix develop path:../../nix#webgpufundamentals`)가 제공합니다. Linux에서는 이 셀이
`PUPPETEER_EXECUTABLE_PATH`를 nixpkgs Chromium으로 설정해 빌드 중 Chrome 다운로드를
건너뜁니다. darwin에서는 기존처럼 원본 lesson-builder가 Puppeteer로 Chrome을
직접 내려받습니다(이미 설치된 Chrome은 `PUPPETEER_EXECUTABLE_PATH`로 지정할 수
있습니다).
빌드는 원본 서브모듈 이력을 읽어 게시 날짜를 생성합니다. 원본 파일은 수정하지
않고 별도 빌드 복사본에 한국어 오버레이와 필요한 수정만 적용합니다.

Pages 경로는 `/webgpufundamentals/`이며 한국어 인덱스로 연결됩니다.
CI는 커밋된 `state/`에서 번역을 재구성합니다. 모든 source 경로를 먼저 검사해
새 문서나 미번역 문서가 있으면 provider를 호출하기 전에 실패합니다.
`ko/`, `ko-metadata/`, `metadata/`, `build/`, `dist/`는 재생성 산출물입니다.

검증은 메타데이터 키, 코드 펜스, 템플릿, WGSL HTML 구조·속성·코드 리터럴,
저장된 평가 이슈, 모든 한국어 페이지의 로컬 링크·자산·앵커를 포함합니다.
URL 또는 변수명에만 등장하는 용어의 오탐은 원문과 번역의 보존 여부를 확인한
좁은 예외로 처리합니다. 번역된 제목에는 영문 앵커 별칭도 유지합니다.

원본의 확인된 링크 오타는 빌드 결과에서 바로잡습니다. 원본에 없는 normal
mapping, skinning, blend targets 문서와 래스터화 도표는 준비 중임을 표시합니다.
별도의 내용을 만들어 채우지 않습니다. 예제 편집기는 Pages 하위 경로에서도
Monaco와 helper 자산을 불러오도록 보정합니다.

테스트:

```sh
python3 -m unittest discover -s scripts -p 'test_*.py'
```

원본 라이선스와 정적 예제·자산은 빌드 산출물에 함께 포함됩니다.
