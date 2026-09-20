# 원고 복원 규칙

이 디렉터리의 `source/`는 Jeff Erickson의 *Algorithms* PDF(`upstream/Algorithms-JeffE.pdf`)에서
복원한 영어 LaTeX 원고입니다. 저자의 원고가 아닙니다.

## 입력

- `build/extract/pNNN.png`: 쪽 이미지(구조의 기준, 150dpi)
- `build/extract/pNNN.json`: 스팬별 텍스트·글꼴·크기·bbox와 분류
  (`text`/`math`/`code`/`heading`/`figure`/`header_footer`), 링크(`name`이
  `figure.2.2`, `section.2.3`, `Hfootnote.48` 같은 원서의 hyperref 목적지)
- `build/extract/chapters.json`: 장별 PDF 쪽 범위(키 `preface`, `00`~`12`,
  `image-credits`, `colophon`)
- `source/figures/manifest.json`: 쪽별 그림 파일(`p091-Im42` → PDF 91쪽의 벡터 그림)

추출물 만들기(한 번):

```sh
cd projects/jeffe-algorithms
nix develop path:../../nix#jeffe-algorithms -c python3 scripts/extract_figures.py upstream/Algorithms-JeffE.pdf source/figures build/extract/figures.json
nix develop path:../../nix#jeffe-algorithms -c python3 scripts/extract.py upstream/Algorithms-JeffE.pdf build/extract build/extract/figures.json
```

## 글꼴로 읽는 원서 텍스트 레이어

| 글꼴 | 분류 | 뜻 |
|---|---|---|
| `XCharter-Roman` | text | 본문. 작은 대문자(`PlaceQueens`, `True`)도 이 글꼴이다 → `\Proc`, `\Const` |
| `XCharter-Italic` | text | 강조, 그리고 **여러 글자 변수**(`legal`, `Splittable(i)`의 함수 이름) → `\Var` |
| `XCharter-BoldItalic` | text | 정의하는 용어 → `\textbf{\textit{…}}` |
| `XCharter-Slanted` | text | 힌트 `[Hint: …]` → `\Hint` |
| `CharterBT-*`, `MathDesign-*` | math | 수식의 글자·숫자·기호. `CharterBT-Bold*`는 굵은 수식 → `\symbfit{…}` |
| `stmary*` | math | 방향 간선 화살표(`⟨ARC⟩`) → `\arc` |
| `Dingbats` | math | 연습문제·소절 표시(`⟨DIFF:♥⟩` 등) → `\difficulty` |
| `Inconsolatazi4-*` | code | 문자열 리터럴 → `\Str`(굵은 파란 글자는 `\StrMark`) |
| `Roboto-*` | heading | 제목, 캡션, 장 첫머리 인용구, 의사코드 주석 |

본문 줄의 스팬은 단어마다 쪼개져 있을 수 있고 공백이 없다. 7.7pt 숫자는 각주 표시,
5.7pt 숫자는 각주 본문 앞 번호다. 원서 텍스트 레이어는 `ff` 합자 뒤의 공백을 잃는다
(`offthe`) — 원고에는 `off the`로 쓴다(검증기가 공백 차이를 무시한다).

작업용 보기: `python3 scripts/pageview.py 89 90 91`은 쪽마다 스팬을 줄로 묶고 글꼴 표시
(`⟦수식⟧`, `_이탤릭_`, `*_굵은 이탤릭_*`, `` `코드` ``, `{sf:로보토}`)와 줄의 y·x·크기를 붙여 보여 준다.
쪽 이미지(구조)와 이 출력(낱말)을 같이 보며 옮겨 쓴다.

## 규칙

1. **단어는 JSON에서 복사한다.** 산문 단어를 이미지에서 새로 타이핑하지 않는다. 합자는
   이미 풀려 있다(`ﬁ`→`fi`). 줄 끝 하이픈 줄바꿈(`algo-`+`rithm`)은 잇는다. 원서의 오타도
   그대로 둔다(예: 2장 `removing removing`, `A[2..n)`).
2. **구조는 이미지를 따른다.** 절·소절, 목록, 장 첫머리 인용구, 의사코드, 수식 정렬, 각주,
   문단 경계(첫 줄 들여쓰기 = 새 문단, 목록·수식·상자 바로 뒤 들여쓰기 없음 = 같은 문단이므로
   빈 줄을 두지 않는다).
3. **매크로 어휘만 쓴다.** 표준 LaTeX(`\textit`, `\textbf`, `\emph`, `\footnote`,
   `enumerate`, `itemize`, `align*`, `cases`, `figure`, `minipage`, `center`, `\mbox` 등)와
   아래 어휘만 쓴다. 새 매크로가 필요하면 먼저 `preamble.tex`와 이 문서를 고친다(병렬 복원
   중에는 직접 고치지 말고 보고한다).

   | 매크로 | 쓰임 |
   |---|---|
   | `\arc{u}{v}` | 방향 간선 u→v(수식 안) |
   | `\Proc{PlaceQueens}` | 절차 이름(작은 대문자). 본문·수식·의사코드·캡션 어디서나 |
   | `\Const{True}` | 상수(`True`, `False`, `None`, `Good`, `Bad` 등 작은 대문자) |
   | `\Var{legal}` | 여러 글자 변수·함수 이름(원서 `XCharter-Italic`). 수식 안에서도 쓴다 |
   | `\Str{ARTISTOIL}` | 빨간 고정폭 문자열 리터럴. `·`(U+00B7)도 그대로 넣는다 |
   | `\StrMark{P}`, `\StrMarkAlt{PP}` | `\Str` 안에서 강조한 글자(파란 굵은 밑줄 / 초록 굵은 윗줄) |
   | `\Hint{…}` | `[Hint: …]`(XCharter 사체). `Hint:`와 괄호는 매크로가 찍는다 |
   | `pseudocode` 환경 | 상자 의사코드(아래 7번) |
   | `\Header{…}` | 의사코드 절차 머리 줄(밑줄, 본문보다 1em 왼쪽) |
   | `\Indent` | 의사코드 들여쓰기 한 단계(2em) |
   | `\Comment{…}` | 의사코드 주석 `⟨⟨…⟩⟩`. 줄 끝이면 상자 오른쪽 끝에 붙고, 줄 맨 앞이면 머리 줄 위치 |
   | `problembox` 환경 | 테두리 친 문제 진술("Given an index $i$, find …") |
   | `\Segments{BLUE,STEM}{HEARTH}` | 문자열 분할 그림(정한 단어들, 검은 막대, 남은 문자열). `\[ … \]` 안에만 |
   | `\Decisions{-3,+1,+4}{?5,8,-2}` | 수열 결정 그림. 막대 왼쪽 `+x` 포함(빨강·노랑)/`-x` 제외(회색), 오른쪽 `?x` 후보(굵게·물음표)/`-x` 회색/`x` 보통. `\[ … \]` 안에만 |
   | `exercises` 환경 | 장 끝 연습문제(`\section*{Exercises}` + 번호 목록) |
   | `\difficulty{…}` | 연습문제·소절 표시. 글자 하나가 기호 하나: `h` 빨간 하트(작게), `H` 큰 빨간 하트, `d` 파란 다이아몬드, `c` 초록 클로버, `s` 검은 스페이드. 원서 7pt 기호는 `h/d/c/s`, 10pt 하트는 `H`. 예: `\difficulty{ch}` = ♣♥ |
   | `\epigraph{인용}{출처}` | 장 첫머리 인용구. `\chapter` **앞**에 원서 순서대로 쓴다. 출처 앞 `—`는 매크로가 찍는다. 출처가 두 줄이면 `\\` |
   | `\figref{fig:2.3}` | "Figure 2.3" |
   | `\secref{sec:2.1}`, `\chapref{ch:1}` | "Section 2.1", "Chapter 1" |
   | `\omitfigure{…}` | 싣지 않는 그림 자리(그림 1.25) |

   수식 관례(unicode-math, `mathrm=sym`):
   - `\gets`, `\emptyset`, `\setminus`, `\neg`, `\vee`/`\wedge`, `\bigvee`, `\le`/`\ge`,
     `\neq`, `\cdots`, `\langle…\rangle`, `\implies`. `\leadsto`(⇝)는 프리앰블이 정의한다.
   - 배열 구간은 `$A[1\,..\,n]$`, `$A[i+1\,..\,n]$`.
   - 수식 안의 영어 낱말은 `\text{if }`, `\text{otherwise}`, `\text{ and }`(번역 대상이 된다).
   - 원서에서 **텍스트 글꼴**인 아래첨자(예: $T_{\text{opt}}$의 `opt`)는 `T_{\textup{opt}}`
     (번역되지 않는 텍스트 글꼴).
   - 굵은 수식은 `\symbfit{…}`(글자 굵은 이탤릭, 숫자 굵게).
   - `\max`, `\min` 같은 연산자 이름은 수학 글꼴로 나온다(원서와 같다).
4. **어휘를 고른 이유(번역 파서):** yeokja `latex` 파서는 `\textit`, `\textsc`, `\textbf`,
   `\emph`, `\text`의 인자를 번역 대상으로 따로 제시하고, `tabular`의 칸도 번역 대상으로
   제시한다. 그래서
   - 변수·상수·절차 이름을 `\textit{legal}`이나 `\textsc{True}`로 쓰면 번역되어 버린다.
     반드시 `\Var`, `\Const`, `\Proc`를 쓴다.
   - 번역하면 안 되는 **보여 주기용 자료**(문자열 예시, 라틴어 연속 표기, 분할·결정 그림)는
     `\[ … \]` 안에 `\mbox{…}`, `\Str`, `\Segments`, `\Decisions`로 둔다. 파서는 여러 줄
     `\[`…`\]`를 통째로 건너뛴다. 이런 표를 `tabular`로 만들지 않는다.
   - `pseudocode` 환경 안은 번역하지 않고 `\Comment{…}` 인자만 번역한다(파서가 `pseudocode`를
     불투명 환경으로, `\Comment`를 가시 텍스트 명령으로 다룬다).
5. **라벨:** `\label{ch:2}`, `\label{sec:2.1}`, `\label{fig:2.3}`, `\label{ex:2.3}`,
   `\label{eq:2.1}`. 원서 번호와 같아야 한다. 본문 참조는 JSON 링크의 `name`으로 확인한다.
   소절(번호 없음)에는 라벨을 달지 않는다.
6. **줄 단위 규칙(파서가 줄 단위로 읽는다):**
   - `\begin{…}`, `\end{…}`, `\caption{…}`, `\label{…}`, `\includegraphics{…}`, `\item …`,
     `\section{…}`, `\subsection{…}`은 각각 **줄 맨 앞**에 둔다. `\begin{figure}[ht]` 줄에
     `\begin{pseudocode}`를 이어 쓰지 않는다.
   - `\caption{…}`는 한 줄에 쓴다(여러 줄이면 첫 줄만 번역 대상이 된다).
   - `\[`와 `\]`는 각각 한 줄에 따로 둔다.
   - 문단 사이는 빈 줄. 문단 안에서는 한 문장 한 줄이 좋다.
7. **의사코드:** `\begin{pseudocode}` … `\end{pseudocode}`. 원서 한 줄이 원고 한 줄, 줄 끝은
   `\\`(마지막 줄 제외). 절차 머리 줄은 `\Header{…}`. 들여쓰기 단계마다 `\Indent`(머리 줄
   바로 아래 줄은 `\Indent` 없음). 환경 안에 빈 줄을 두지 않는다. 줄을 `[`로 시작하지 않는다.
   키워드(if, else, for, to, while, return, print, and, or …)는 그냥 글자로, 수식은 `$…$`,
   여러 글자 변수는 `\Var`, 상수는 `\Const`, 절차는 `\Proc`. 주석은 `\Comment{…}`.
   원서처럼 그림 안에 있으면 `figure` 안에 `pseudocode`를 두고 `\caption`을 단다. 상자 두 개를
   나란히 두려면 `center` 안에 `minipage` 두 개(`[c]{0.55\linewidth}`, `\hfill`).
   ```latex
   \begin{figure}[ht]
   \centering
   \begin{pseudocode}
   \Header{\Proc{PlaceQueens}$(Q[1\,..\,n], r)$:}\\
   if $r = n+1$\\
   \Indent print $Q[1\,..\,n]$\\
   else\\
   \Indent for $j \gets 1$ to $n$\\
   \Indent\Indent $\Var{legal} \gets \Const{True}$\\
   …
   \Indent\Indent\Indent \Proc{PlaceQueens}$(Q[1\,..\,n], r+1)$ \Comment{Recursion!}
   \end{pseudocode}
   \caption{Gauss and Laquière’s backtracking algorithm for the $n$ queens problem.}
   \label{fig:2.2}
   \end{figure}
   ```
   절차 위에 붙은 주석 줄은 `\Comment{Does any subset of $X$ sum to $T$?}\\`처럼 머리 줄 앞에 쓴다.
8. **그림:** `\begin{figure}[ht]` / `\centering` / `\includegraphics{p091-Im42}` / `\caption{…}` /
   `\label{fig:2.3}` / `\end{figure}`. 한 그림이 파일 여러 개면 원서 배치대로 나란히 둔다
   (`\hfil`, `\\[1ex]`). 그림은 원서 크기 그대로 넣는다(`\includegraphics`에 크기 옵션 없음).
   캡션 안의 숫자가 원서에서 Roboto(텍스트)면 수식으로 쓰지 않는다: `3${}\times{}$3`.
9. **그림 1.25**는 파일이 없다. `\omitfigure{Figure 1.25 (a portrait of the author) is omitted; see page 61 of the original.}`를 넣고 캡션과 라벨은 유지한다.
10. **각주**는 `\footnote{…}`를 원서의 표시 위치에 둔다(표시 번호의 bbox 위치로 앞 낱말을 찾는다).
11. **연습문제**는 장 끝 `\begin{exercises}` / `\item …` / `\end{exercises}`, 하위 문항은
    `\begin{enumerate}`(라벨 (a), (b)는 자동). 표시는 `\item \difficulty{h} …`처럼 `\item` 바로
    뒤에 둔다. 번호 뒤에 바로 하위 문항이 오면(`4. (a) …`) `\item` 다음 줄에 `\begin{enumerate}`.
    하위 문항 안의 새 문단(원서에서 들여쓴 줄)은 빈 줄로 나눈다.
12. **표시가 붙은 절·소절:** `\subsection{\difficulty{H}Analysis}`,
    `\section{\difficulty{h}Linear-Time Selection}`.
13. 쪽 나눔·줄 나눔을 흉내 내지 않는다(`\newpage`, `\\` 금지 — 의사코드·인용구 출처·
    배열 안은 예외).
14. 한 장이 끝나면 `scripts/verify_source.py <장 키>`를 통과시킨다.

## 검증

```sh
cd projects/jeffe-algorithms
nix develop path:../../nix#jeffe-algorithms -c python3 scripts/verify_source.py 02
```

- 장 하나만 영어 모드로 컴파일해(`\includeonly`, 차례 생략) 텍스트를 뽑고 원서의 같은 장과
  단어 단위로 대조한다. 보고서는 `build/verify/<키>.txt`, 복원본 PDF는 `build/verify/<키>/main.pdf`.
- 정규화: 공백·스팬 경계 무시, 문장부호는 따로 토큰, 줄 끝 하이픈 연결과 복합어 하이픈 제거,
  `page(s) N` → `#`, 합자·곧은/굽은 따옴표 통일, 풀리지 않은 참조 `??` = 번호.
  원서의 `ff` 합자 뒤 공백 손실과 Roboto 작은 대문자(원서는 대문자로 추출)는 같은 것으로 본다.
- **떠다니는 요소:** 그림·각주는 쪽 나눔이 달라 텍스트 흐름의 다른 자리에 나온다. 4토큰 이상
  같은 덩어리는 "moved"로 맞춘 것으로 치고 보고서에 따로 적는다. 남은 불일치 가운데 순서만
  다른 것은 `residual after ignoring order`가 비어 있는 것으로 알 수 있다.
- **기준(실패하면 종료 코드 1):**
  - 원서 토큰 중 불일치 비율 ≤ 0.5%.
  - 구조: 그림 캡션 번호 목록, 각주 수(`Hfootnote` 링크), 연습문제 번호 목록, 절·소절 목록
    (두 PDF의 목차 outline), 원서 Index of Pseudocode의 이 장 절차가 모두 원고의 `\Proc`에 있음.
- **검토(사람·에이전트):** 보고서의 모든 hunk와 `residual after ignoring order`를 원서 쪽 이미지와
  대조해 원고 오류면 고친다. 수식 기호열 비교(`math symbols`, 검토용)의 hunk도 모두 본다.
  남는 수식 hunk는 큰 괄호·첨자 순서 차이뿐이어야 한다. 수식이 많은 쪽은 원서·복원본 쪽을
  나란히 렌더링해 눈으로 확인한다: `nix develop path:../../nix#jeffe-algorithms -c python3
  scripts/sidebyside.py 02` → `build/sbs/02-NN.png`(왼쪽 원서, 오른쪽 복원본).

## 파일럿(2장 Backtracking, PDF 89–114쪽) 결과

- **단어 대조:** 불일치 **0.1238%**(원서 9,694토큰, 복원 9,696토큰). 남은 8개 hunk는 모두 각주
  3·8이 쪽 나눔 때문에 다른 자리에 나온 순서 차이(`residual after ignoring order: missing=[] extra=[]`).
  moved 13덩어리도 모두 각주·그림 위치 차이.
- **구조:** 그림 6(2.1–2.6), 각주 14, 연습문제 6, 절·소절 16항목, Index of Pseudocode 8절차 모두 일치.
- **수식:** 기호열 불일치 0.52%(23 hunk; 큰 괄호·중괄호 조각과 첨자 추출 순서). 26쪽을 나란히
  렌더링해 확인한 결과 수식 내용 오류 0, 굵은 수식 누락 2(89쪽 $n$, 111쪽 $T(n)=O(3^n)$ → `\symbfit`).
- **복원본 쪽수:** 26쪽(원서와 같음; `microtype` 사용 시).
- **소요 시간:** 초안 작성 약 40분(쪽당 약 1.5분, 어휘를 만들면서), 검증 도구 보정과 나란히 보기
  검토 약 45분(쪽당 약 1.7분). 도구가 준비된 뒤라면 장 하나에 쪽당 약 2–3분이 예상된다.
- **어휘 조정(파일럿에서 추가·변경):** 의사코드 환경 이름 `algorithm` → `pseudocode`(yeokja `latex`
  파서가 `pseudocode`를 불투명 환경으로, `\Comment`를 가시 텍스트 명령으로 다루도록 확장; 흔한 `algorithm`
  float의 캡션을 숨기지 않으려고 이름을 따로 둠), `\Header`, `\StrMark`/`\StrMarkAlt`, `\Segments`, `\Decisions`,
  `problembox`; `\Hint`는 XCharter 사체; `\Comment`는 줄 끝이면 오른쪽 정렬;
  `\difficulty{n}`(숫자) 대신 기호 글자 목록(`h/H/d/c/s`); 텍스트 아래첨자 `\textup`, 굵은 수식
  `\symbfit`; unicode-math `mathrm=sym`, `\leadsto` 별칭, `microtype`.
- **번역·한국어 빌드(파일럿):** 2장 326세그먼트(의사코드 주석 9개 포함)를 `claude_code`/`claude-sonnet-5`로
  약 5분에 번역, 기계 평가 0건, `status --check` 통과. 의사코드 키워드는 영어로 남고 주석만 번역됨.
  한국어 PDF(LuaLaTeX + luatexko) 빌드 성공, 누락 글리프·미해결 참조 없음. 번역문이 ASCII `"…"`를
  쓰면 닫는 따옴표로만 찍히므로 `scripts/prepare_pdf.py`가 조립 트리에서 `“…”`로 바꾼다. 나눔 글꼴에
  이탤릭이 없어 한국어 `\textit`은 가짜 기울임(FakeSlant)으로 조판한다.
- **기준 조정:** 0.5% 기준은 그대로. 대조 방식만 보강(떠다니는 요소 이동 허용, 공백·작은 대문자 정규화,
  순서 무시 잔차 표시, 구조 개수 자동 대조).
