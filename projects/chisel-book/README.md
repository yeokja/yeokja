# Digital Design with Chisel 한국어 번역

이 디렉터리는 [schoeberl/chisel-book](https://github.com/schoeberl/chisel-book)의
`chisel-book.tex` 원고를 yeokja로 한국어 번역하는 프로젝트입니다. 원문은
`upstream/` 서브모듈에 두고, 번역 상태는 `state/`에 저장합니다. `ko/`는 상태에서
재구성되는 파생 출력이므로 Git에 커밋하지 않습니다.

번역 프로바이더는 Claude Code의 `claude-sonnet-5`이며, 용어·링크·LaTeX 형식의
기계적 평가는 번역 중 자동으로 실행합니다.

```sh
../../target/debug/yeokja inspect upstream
../../target/debug/yeokja coverage upstream --min-lines 3
../../target/debug/yeokja translate upstream
../../target/debug/yeokja status upstream --check
../../target/debug/yeokja evaluate upstream --mechanical-only
../../target/debug/yeokja orphans
../../target/debug/yeokja build pdf
```

PDF 빌드에는 Python 3, XeLaTeX, BibTeX, MakeIndex와 나눔 글꼴이 필요합니다.

원문은 Creative Commons Attribution-ShareAlike 4.0 International License로
배포됩니다. 이 번역은 비공식 번역이며 원저작자는 Martin Schoeberl입니다.
