<?xml version="1.0" encoding="UTF-8" ?>

<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform">

<!--********************************************************************
Copyright 2014-2016 Robert A. Beezer

This file is part of MathBook XML.

MathBook XML is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 2 or version 3 of the
License (at your option).

MathBook XML is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with MathBook XML.  If not, see <http://www.gnu.org/licenses/>.
*********************************************************************-->

<!-- See  xsl/localizations/README.md  for an explanation of this file -->

<!-- ko-KR, Korean (Republic of Korea)                                  -->
<!-- Translated from en-US.xsl for the yeokja Korean edition of          -->
<!-- Interactive Linear Algebra (https://github.com/yeokja/yeokja).        -->
<!-- Same license as the file it translates (GPL version 2 or 3).        -->

<xsl:variable name="ko-KR">
    <!-- THEOREM-LIKE blocks -->
    <!-- Environments which have proofs, plus proofs themselves -->
    <localization string-id='theorem'><xsl:text>정리</xsl:text></localization>
    <localization string-id='corollary'><xsl:text>따름정리</xsl:text></localization>
    <localization string-id='lemma'><xsl:text>보조정리</xsl:text></localization>
    <localization string-id='algorithm'><xsl:text>알고리즘</xsl:text></localization>
    <localization string-id='proposition'><xsl:text>명제</xsl:text></localization>
    <localization string-id='claim'><xsl:text>주장</xsl:text></localization>
    <localization string-id='fact'><xsl:text>사실</xsl:text></localization>
    <localization string-id='identity'><xsl:text>항등식</xsl:text></localization>
    <localization string-id='proof'><xsl:text>증명</xsl:text></localization>
    <localization string-id='case'><xsl:text>경우</xsl:text></localization>
    <!-- Components of the narrative -->
    <!-- Mathematical statements without proofs -->
    <!-- AXIOM-LIKE blocks -->
    <localization string-id='axiom'><xsl:text>공리</xsl:text></localization>
    <localization string-id='conjecture'><xsl:text>추측</xsl:text></localization>
    <localization string-id='principle'><xsl:text>원리</xsl:text></localization>
    <localization string-id='heuristic'><xsl:text>발견법</xsl:text></localization>
    <localization string-id='hypothesis'><xsl:text>가설</xsl:text></localization>
    <localization string-id='assumption'><xsl:text>가정</xsl:text></localization>
    <!-- Definitions -->
    <localization string-id='definition'><xsl:text>정의</xsl:text></localization>
    <localization string-id='essential'><xsl:text>정의</xsl:text></localization>
    <!-- Single Line Mathematics -->
    <localization string-id='me'><xsl:text>식</xsl:text></localization>
    <localization string-id='men'><xsl:text>식</xsl:text></localization>
    <localization string-id='mrow'><xsl:text>식</xsl:text></localization>
    <!-- Display Mathematics -->
    <localization string-id='md'><xsl:text>디스플레이 수식</xsl:text></localization>
    <localization string-id='mdn'><xsl:text>디스플레이 수식</xsl:text></localization>
    <!-- Types of documents, mostly for informational messages -->
    <localization string-id='volume'><xsl:text>권</xsl:text></localization>
    <localization string-id='book'><xsl:text>책</xsl:text></localization>
    <localization string-id='article'><xsl:text>글</xsl:text></localization>
    <localization string-id='letter'><xsl:text>편지</xsl:text></localization>
    <localization string-id='memo'><xsl:text>메모</xsl:text></localization>
    <localization string-id='presentation'><xsl:text>발표</xsl:text></localization>
    <!-- Parts of a document -->
    <localization string-id='frontmatter'><xsl:text>앞부분</xsl:text></localization>
    <localization string-id='part'><xsl:text>부</xsl:text></localization>
    <localization string-id='chapter'><xsl:text>장</xsl:text></localization>
    <localization string-id='appendix'><xsl:text>부록</xsl:text></localization>
    <localization string-id='section'><xsl:text>절</xsl:text></localization>
    <localization string-id='subsection'><xsl:text>소절</xsl:text></localization>
    <localization string-id='subsubsection'><xsl:text>하위 소절</xsl:text></localization>
    <localization string-id='introduction'><xsl:text>들어가며</xsl:text></localization>
    <localization string-id='conclusion'><xsl:text>맺으며</xsl:text></localization>
    <localization string-id='exercises'><xsl:text>연습문제</xsl:text></localization>
    <localization string-id='references'><xsl:text>참고 문헌</xsl:text></localization>
    <localization string-id='backmatter'><xsl:text>뒷부분</xsl:text></localization>
    <!-- paragraph is deprecated, getting plural correct is not super critical, just in messages -->
    <localization string-id='paragraphs'><xsl:text>문단</xsl:text></localization>
    <localization string-id='subparagraph'><xsl:text>하위 문단</xsl:text></localization>
    <!-- Components of the narrative -->
    <!-- REMARK-LIKE blocks -->
    <!-- "note" is used within "biblio", likely to change -->
    <localization string-id='remark'><xsl:text>참고</xsl:text></localization>
    <localization string-id='convention'><xsl:text>규약</xsl:text></localization>
    <localization string-id='note'><xsl:text>노트</xsl:text></localization>
    <localization string-id='observation'><xsl:text>관찰</xsl:text></localization>
    <localization string-id='warning'><xsl:text>주의</xsl:text></localization>
    <localization string-id='bluebox'><xsl:text>중요</xsl:text></localization>
    <localization string-id='insight'><xsl:text>통찰</xsl:text></localization>
    <localization string-id='computation'><xsl:text>계산</xsl:text></localization>
    <localization string-id='technology'><xsl:text>기술</xsl:text></localization>
    <!-- ASIDE-LIKE blocks -->
    <localization string-id='aside'><xsl:text>여담</xsl:text></localization>
    <localization string-id='biographical'><xsl:text>인물 여담</xsl:text></localization>
    <localization string-id='historical'><xsl:text>역사 여담</xsl:text></localization>
    <!-- EXAMPLE-LIKE blocks -->
    <localization string-id='example'><xsl:text>예제</xsl:text></localization>
    <localization string-id='specialcase'><xsl:text>예제</xsl:text></localization>
    <localization string-id='question'><xsl:text>질문</xsl:text></localization>
    <localization string-id='problem'><xsl:text>문제</xsl:text></localization>
    <!-- PROJECT-LIKE blocks -->
    <localization string-id='project'><xsl:text>프로젝트</xsl:text></localization>
    <localization string-id='activity'><xsl:text>활동</xsl:text></localization>
    <localization string-id='exploration'><xsl:text>탐구</xsl:text></localization>
    <localization string-id='task'><xsl:text>과제</xsl:text></localization>
    <localization string-id='investigation'><xsl:text>조사</xsl:text></localization>
    <!-- assemblages are collections of minimally structured material -->
    <localization string-id='assemblage'><xsl:text>모음</xsl:text></localization>
    <localization string-id='poem'><xsl:text>시</xsl:text></localization>
    <!-- Objectives is the block, objective is a list item within -->
    <localization string-id='objectives'><xsl:text>목표</xsl:text></localization>
    <localization string-id='objective'><xsl:text>목표</xsl:text></localization>
    <!--  -->
    <localization string-id='figure'><xsl:text>그림</xsl:text></localization>
    <localization string-id='table'><xsl:text>표</xsl:text></localization>
    <localization string-id='listing'><xsl:text>목록</xsl:text></localization>
    <localization string-id='fn'><xsl:text>각주</xsl:text></localization>
    <localization string-id='contributor'><xsl:text>기여자</xsl:text></localization>
    <!-- Lists and their items -->
    <localization string-id='list'><xsl:text>목록</xsl:text></localization>
    <localization string-id='li'><xsl:text>항목</xsl:text></localization>
    <!-- A regular paragraph, not the old sectioning structure -->
    <localization string-id='p'><xsl:text>문단</xsl:text></localization>
    <localization string-id='blockquote'><xsl:text>인용</xsl:text></localization>
    <!-- Parts of an exercise and it's solution -->
    <localization string-id='exercise'><xsl:text>연습문제</xsl:text></localization>
    <localization string-id='webwork'><xsl:text>WeBWorK</xsl:text></localization>
    <localization string-id='hint'><xsl:text>힌트</xsl:text></localization>
    <localization string-id='answer'><xsl:text>답</xsl:text></localization>
    <localization string-id='solution'><xsl:text>풀이</xsl:text></localization>
    <!-- A group of sectional exercises (with introduction and conclusion) -->
    <localization string-id='exercisegroup'><xsl:text>연습문제 묶음</xsl:text></localization>
    <!-- Bibliographic items (note is distinct from sidebar "Annotations" below) -->
    <localization string-id='biblio'><xsl:text>참고 문헌 항목</xsl:text></localization>
    <!-- Front matter components -->
    <localization string-id='toc'><xsl:text>차례</xsl:text></localization>
    <localization string-id='abstract'><xsl:text>초록</xsl:text></localization>
    <localization string-id='preface'><xsl:text>머리말</xsl:text></localization>
    <localization string-id='acknowledgement'><xsl:text>감사의 글</xsl:text></localization>
    <localization string-id='biography'><xsl:text>저자 소개</xsl:text></localization>
    <!-- singular and plural titles for biography subdivision -->
    <localization string-id='about-author'><xsl:text>저자 소개</xsl:text></localization>
    <localization string-id='about-authors'><xsl:text>저자 소개</xsl:text></localization>
    <localization string-id='foreword'><xsl:text>추천사</xsl:text></localization>
    <localization string-id='dedication'><xsl:text>헌사</xsl:text></localization>
    <localization string-id='colophon'><xsl:text>판권</xsl:text></localization>
    <!-- Back matter components -->
    <!-- index-part is deprecated, but not abandoned          -->
    <!-- NB: repurpose translations, maybe move appendix here -->
    <localization string-id='index-part'><xsl:text>찾아보기</xsl:text></localization>
    <localization string-id='jump-to'><xsl:text>바로 가기:</xsl:text></localization>
    <!-- Parts of the Index -->
    <localization string-id='index'><xsl:text>찾아보기</xsl:text></localization>
    <localization string-id='see'><xsl:text>참조</xsl:text></localization>
    <localization string-id='also'><xsl:text>함께 보기</xsl:text></localization>
    <!-- Notation List headings/foot -->
    <localization string-id='symbol'><xsl:text>기호</xsl:text></localization>
    <localization string-id='description'><xsl:text>설명</xsl:text></localization>
    <localization string-id='location'><xsl:text>위치</xsl:text></localization>
    <localization string-id='page'><xsl:text>쪽</xsl:text></localization>
    <localization string-id='continued'><xsl:text>다음 쪽에 계속</xsl:text></localization>
    <!-- Navigation Interface elements -->
    <!-- Assistive "skip to content" link -->
    <localization string-id='skip-to-content'><xsl:text>본문으로 건너뛰기</xsl:text></localization>
    <localization string-id='previous'><xsl:text>이전</xsl:text></localization>
    <localization string-id='up'><xsl:text>위로</xsl:text></localization>
    <localization string-id='next'><xsl:text>다음</xsl:text></localization>
    <!-- Keep these short, so buttons are not overly wide, 4 characters maximum -->
    <localization string-id='previous-short'><xsl:text>이전</xsl:text></localization>
    <localization string-id='up-short'><xsl:text>위</xsl:text></localization>
    <localization string-id='next-short'><xsl:text>다음</xsl:text></localization>
    <!-- NB: Use toc from above for both headings and navigation sidebar-->
    <localization string-id='annotations'><xsl:text>주석</xsl:text></localization>
    <localization string-id='feedback'><xsl:text>의견</xsl:text></localization>
    <localization string-id='authored'><xsl:text>작성 도구:</xsl:text></localization>
    <!-- Parts of memos and letters -->
    <localization string-id='to'><xsl:text>받는 사람</xsl:text></localization>
    <localization string-id='from'><xsl:text>보낸 사람</xsl:text></localization>
    <localization string-id='subject'><xsl:text>제목</xsl:text></localization>
    <localization string-id='date'><xsl:text>날짜</xsl:text></localization>
    <localization string-id='copy'><xsl:text>참조</xsl:text></localization>
    <localization string-id='enclosure'><xsl:text>첨부</xsl:text></localization>
    <!-- Various -->
    <localization string-id='todo'><xsl:text>할 일</xsl:text></localization>
    <localization string-id='editor'><xsl:text>편집자</xsl:text></localization>
    <localization string-id='edition'><xsl:text>판</xsl:text></localization>
    <localization string-id='website'><xsl:text>웹사이트</xsl:text></localization>
    <localization string-id='copyright'><xsl:text>저작권</xsl:text></localization>
    <!-- HTML clickables (lowercase strings to click on) -->
    <localization string-id='permalink'><xsl:text>고유 링크</xsl:text></localization>
    <localization string-id='incontext'><xsl:text>본문에서 보기</xsl:text></localization>
    <!-- Sage Cell evaluate button      -->
    <!-- eg, "Evaluate (Maxima)"        -->
    <!-- 2017-05-14: 'code' is obsolete -->
    <localization string-id='evaluate'><xsl:text>실행</xsl:text></localization>
    <!-- <localization string-id='code'><xsl:text>코드</xsl:text></localization> -->
</xsl:variable>

</xsl:stylesheet>
