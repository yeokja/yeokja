# 재무관리 원리(Principles of Finance 2e) 한국어 번역

OpenStax(Rice University)가 펴낸 개방형 교재 *Principles of Finance 2e*(Julie Dahlquist,
Rainford Knight)를 한국어로 옮긴 비공식 번역입니다. 원문은
[openstax/osbooks-principles-finance](https://github.com/openstax/osbooks-principles-finance)의
CNXML 원고이며 `upstream` 서브모듈로 고정합니다. [yeokja](https://github.com/yeokja/yeokja)와
함께 Anthropic 사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한
상태로 작업하였습니다.

게시 주소: <https://yeokja.moreal.dev/principles-finance/>

## 범위

- 번역 대상: `upstream/modules/*/index.cnxml` 가운데 컬렉션에 없는 자리 채움 모듈
  `m00088`을 뺀 131개 모듈과 `upstream/collections/principles-finance-2e.collection.xml`
  (책·장 제목). 본문·제목·캡션·노트·각주·용어집, 그림 `alt`, 표 `summary`, 수식 안의
  `mtext` 단어를 `cnxml` 파서로 번역합니다.
- 그대로 두는 것: 수식 식별자, 엑셀 수식과 R 코드, 계산기 키 이름(`2ND`, `ENTER`),
  URL, 그림 속 글자(엑셀·재무제표 화면 포함).
- 각 절 끝의 객관식·복습·연습 문제와 동영상 활동 문제 580개는 원고에 본문이 없고
  OpenStax 연습문제 서비스가 제공합니다. 이 번역에 싣지 않고 빌드가 원문 장 끝 페이지로
  가는 링크 상자로 바꿉니다. 원고에 본문이 있는 보기 문제(생각해 보기)는 번역합니다.
- 용어는 국내 재무관리 교재의 관례를 따릅니다(`glossary.toml`, `terms.toml`).
  재무상태표, 화폐의 시간가치, 순현재가치, 가중평균자본비용처럼 옮기고 약어(NPV, IRR,
  WACC, CAPM)는 그대로 둡니다.

## 라이선스

원서는 [CC BY-NC-SA 4.0](https://creativecommons.org/licenses/by-nc-sa/4.0/deed.ko)
(`upstream/LICENSE`)입니다. 이 번역은 같은 라이선스를 따르는 비상업적 2차적 저작물이며,
원 저작자나 OpenStax의 공식 번역이 아니고 그 보증을 받지 않았습니다. 웹 판의 모든 페이지
하단에 저작자, 그 페이지의 원문 링크, 라이선스, 번역·수정 사실을 표시합니다. OpenStax
로고와 표지 이미지는 쓰지 않습니다.

## 재현

```sh
git submodule update --init projects/principles-finance/upstream
cd projects/principles-finance
../../target/release/yeokja translate upstream        # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream
nix develop path:../../nix#principles-finance -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의 원천이므로
커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와 원문에서 언제든 재생성되므로 커밋하지
않습니다.

OpenStax의 공식 빌드(enki)는 쓰지 않습니다. `scripts/build_site.py`(Python 3 + lxml)가
번역된 CNXML을 정적 사이트로 그립니다. 장·절·그림·표 번호와 빈 상호 참조("그림 1.2"),
노트 라벨, 장 끝 페이지(요약, 핵심 용어, CFA Institute, 문제, 동영상 활동), 각주,
MathML 수식(MathJax 보조), 이전·다음 탐색, 랜딩 목차를 만듭니다.

검증(빌드 명령의 앞뒤에서도 돕니다):

```sh
nix develop path:../../nix#principles-finance -c python3 scripts/check.py skeleton upstream build/tree   # 번역문의 요소 구조가 원문과 같은지
nix develop path:../../nix#principles-finance -c python3 scripts/check.py site build/tree/site          # 내부 링크·앵커·이미지 실재
```

Deploy Pages 워크플로는 커밋된 번역 상태로 `ko/`를 재구성하고 미번역 여부를 검사한 뒤
빌드해, `dist-principles-finance` 산출물을 Pages의 `principles-finance/`에 배치합니다.
