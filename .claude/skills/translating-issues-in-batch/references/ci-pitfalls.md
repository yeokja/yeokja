# CI(Linux, GitHub Actions)에서만 드러나는 결함

macOS는 파일 시스템이 대소문자를 무시하고 로컬 빌드가 다 통과하므로, 다음은 push 전에
스캔하거나 첫 Deploy Pages 실행에서 확인한다. Deploy는 실패한 프로젝트를 이전 배포분으로
보존하므로 전체는 success로 끝나고 해당 경로만 404가 된다 — 잡 단위 conclusion을 본다.

| 증상 | 원인 | 처방 |
|---|---|---|
| `Cannot find the file(s): "Figures/x.png"`, 자원 누락 | 참조 경로와 실제 파일명의 대소문자 불일치 | `git ls-files` 정확한 이름과 참조를 대조하는 스캔; 빌드 전처리에서 참조를 실제 이름으로 바꾸거나 실제 이름으로 복사. 원문 보존 검사가 그 손질을 양쪽 동일하게 보게 한다 |
| `overlay <...>/overlay/ko does not exist` | 프로젝트 `.gitignore`의 `ko/`가 `overlay/ko/`도 가려 커밋에서 빠짐 | 패턴을 `/ko/`로 고정, `git status --ignored`로 overlay 아래 ignored 파일 확인 |
| XSLT/빌드가 원문 안의 서브모듈 파일을 못 찾음 | CI는 `projects/<p>/upstream`만 비재귀로 받음 | pages-projects.json에 `nested_submodules` |
| R/Python 패키지 소스 빌드로 잡이 매우 느림 | nixpkgs 바이너리 캐시에 없는 override | 첫 실행 시간을 보고 캐시 가능한 조합으로 조정 |
| 로컬에서 `claude -p` 한 번이 수 분~수십 분 | 호스트 메모리 압박(오래된 세션 프로세스) | `duration_api_ms`(수 초)와 벽시계 비교로 진단; yeokja 설정 문제가 아님 |

스캔 예(원문 참조 vs git 파일명):
```bash
git -C upstream ls-files > /tmp/exact.txt
grep -rhoE '(Figures|Quizzes|Data)/[A-Za-z0-9_./-]+' upstream --include='*.Rmd' --include='*.html' \
  | sort -u | while read r; do grep -qxF "$r" /tmp/exact.txt || grep -qixF "$r" /tmp/exact.txt && echo "CASE: $r"; done
```
