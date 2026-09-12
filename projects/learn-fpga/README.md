# learn-fpga 한국어 번역

[BrunoLevy/learn-fpga](https://github.com/BrunoLevy/learn-fpga)의 Markdown 문서를
yeokja로 한국어 번역합니다. 원본은 `upstream` 서브모듈에 고정하며, 번역 상태는
`state/`에 커밋합니다. Verilog·C 코드와 이미지 등은 원본을 그대로 제공합니다.

게시 주소: <https://yeokja.moreal.dev/learn-fpga/>

## 번역과 빌드

저장소 루트에서 실행합니다. 번역에는 기존 프로젝트와 동일한 Claude CLI가 필요하고,
HTML 빌드는 `cd projects/learn-fpga && nix develop path:../../nix#learn-fpga`로
준비한 셸(Python 3, Pandoc)에서 실행합니다.

```sh
git submodule update --init projects/learn-fpga/upstream
target/release/yeokja -C projects/learn-fpga translate upstream
target/release/yeokja -C projects/learn-fpga status --check upstream
target/release/yeokja -C projects/learn-fpga build html
python3 projects/learn-fpga/scripts/test_build_site.py
```

`ko/`를 원본 위에 조립하고 Markdown을 `dist/site/`의 HTML로 렌더링합니다.
README는 `index.html`로 바꾸며, 상대 문서 링크와 원본 저장소의 문서 링크를
한국어 페이지로 연결합니다. 제목의 ID는 원문에서 가져와 영어 앵커를 유지합니다.
코드 파일과 라이선스 고지는 함께 복사합니다. CI는 전체 원본 경로의 번역 상태를
검사하므로 누락된 문서가 있으면 배포 전 단계에서 실패합니다.

## 라이선스와 범위

원문의 BSD 3-Clause 라이선스와 각 하위 디렉터리의 별도 라이선스를 유지합니다.
`upstream/LICENSE` 및 개별 파일의 고지를 참고하세요. 원문 저자와 기여자의
공식 번역이 아닌 기계 번역이며, 실행 예제와 코드 주석은 번역 대상에 넣지 않습니다.
Markdown으로 된 라이선스의 번역본은 참고용이며 원문 고지를 함께 제공합니다.
