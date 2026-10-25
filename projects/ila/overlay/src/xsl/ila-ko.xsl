<?xml version='1.0'?>

<!-- yeokja Korean edition of Interactive Linear Algebra.                -->
<!-- Imports the book's own stylesheet (src/xsl/mathbook-html.xsl) and   -->
<!-- overrides only what the Korean web edition changes: interface text  -->
<!-- hard-coded in English, the feedback link (to the translation's      -->
<!-- tracker), the PDF link and cover animation (not built), the bundled -->
<!-- CSS/JS paths, and the MathBox demos (served from the original site). -->
<!-- Same license as the stylesheet it imports (GPL).                    -->

<!DOCTYPE xsl:stylesheet [
    <!ENTITY % entities SYSTEM "../../mathbook/xsl/entities.ent">
    %entities;
]>

<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0"
    xmlns:exsl="http://exslt.org/common"
    extension-element-prefixes="exsl"
>

<xsl:import href="mathbook-html.xsl" />

<!-- Where the original edition serves its MathBox demos. -->
<xsl:param name="ila.demos.base" select="'https://textbooks.math.gatech.edu/ila/'" />
<!-- Where readers report problems with the translation. -->
<xsl:param name="ko.feedback" select="'https://github.com/yeokja/yeokja/issues/new?title=%5Bila%5D%20'" />

<xsl:template name="combined-js">
    <script src="js/ila.js"></script>
</xsl:template>

<xsl:template name="css">
    <link href="css/ila.css" rel="stylesheet" type="text/css"/>
    <link rel="manifest" href="manifest.json"/>
    <link rel="shortcut icon" href="images/logo.gif"/>
</xsl:template>

<xsl:template name="brand-logo">
    <xsl:choose>
        <xsl:when test="/mathbook/docinfo/brandlogo">
            <a id="logo-link" href="{$theme.sponsor}" target="_blank" rel="noopener">
                <img src="{/mathbook/docinfo/brandlogo/@source}" alt="Georgia Tech 로고"/>
            </a>
        </xsl:when>
        <xsl:otherwise>
            <a id="logo-link" href="" />
        </xsl:otherwise>
    </xsl:choose>
</xsl:template>

<xsl:template match="*" mode="primary-navigation">
  <nav id="gt-navbar" class="navbar container" style="">
    <div class="dropdown">
      <div class="toc-border-container" id="toc">
        <div class="toc-contents">
          <xsl:apply-templates select="." mode="toc-items" />
        </div>
      </div>
    </div>
    <div class="navbar-top-buttons toolbar">
      <div class="toolbar-buttons-left">
        <!-- Toggle button -->
        <a class="toggle-button button toolbar-item"
           title="메뉴 열기/닫기" alt="메뉴 열기/닫기"
           href="javascript:void(0)"></a>
        <!-- A page either has an/the index as    -->
        <!-- a child, and gets the "jump to" bar, -->
        <!-- or it deserves an index button       -->
        <xsl:choose>
          <xsl:when test="index-list">
            <div class="toolbar-item">
              <xsl:apply-templates select="." mode="index-jump-nav" />
            </div>
          </xsl:when>
          <xsl:otherwise>
            <xsl:apply-templates select="." mode="index-button" />
          </xsl:otherwise>
        </xsl:choose>
      </div>
      <div class="toolbar-buttons-right">
        <!-- Each button gets an id for keypress recognition/action -->
        <xsl:apply-templates select="." mode="previous-button">
          <xsl:with-param name="id-label" select="'previousbutton-top'" />
        </xsl:apply-templates>
        <xsl:if test="$nav-upbutton='yes'">
          <xsl:apply-templates select="." mode="up-button">
            <xsl:with-param name="id-label" select="'upbutton-top'" />
          </xsl:apply-templates>
        </xsl:if>
        <xsl:apply-templates select="." mode="next-button">
          <xsl:with-param name="id-label" select="'nextbutton-top'" />
        </xsl:apply-templates>
      </div>
    </div>
    <div class="navbar-bottom-buttons toolbar">
      <a class="toggle-button button toolbar-item"
         title="메뉴 열기/닫기" alt="메뉴 열기/닫기"
         href="javascript:void(0)"></a>
      <xsl:apply-templates select="." mode="previous-button">
        <xsl:with-param name="id-label" select="'previousbutton-bot'" />
      </xsl:apply-templates>
      <xsl:if test="$nav-upbutton='yes'">
        <xsl:apply-templates select="." mode="up-button">
          <xsl:with-param name="id-label" select="'upbutton-bot'" />
        </xsl:apply-templates>
      </xsl:if>
      <xsl:apply-templates select="." mode="next-button">
        <xsl:with-param name="id-label" select="'nextbutton-bot'" />
      </xsl:apply-templates>
    </div>
  </nav>
</xsl:template>

<xsl:template match="*" mode="heading-full">
    <xsl:param name="important"/>
    <xsl:element name="h5">
        <xsl:attribute name="class">
            <xsl:text>heading</xsl:text>
            <xsl:if test="$important">
               <xsl:text> important</xsl:text>
            </xsl:if>
        </xsl:attribute>
        <xsl:if test="$important">
            <xsl:element name="img">
                <xsl:attribute name="src">
                    <xsl:text>images/important.svg</xsl:text>
                </xsl:attribute>
                <xsl:attribute name="class">
                    <xsl:text>important</xsl:text>
                </xsl:attribute>
                <xsl:attribute name="alt">느낌표</xsl:attribute>
            </xsl:element>
        </xsl:if>
        <xsl:variable name="the-number">
            <xsl:apply-templates select="." mode="number" />
        </xsl:variable>
        <span class="type">
            <xsl:apply-templates select="." mode="type-name" />
        </span>
        <xsl:if test="not($the-number='')">
            <span class="codenumber">
                <xsl:value-of select="$the-number" />
            </span>
        </xsl:if>
        <xsl:if test="title">
            <span class="title">
                <xsl:text>(</xsl:text>
                <xsl:apply-templates select="." mode="title-full" />
                <xsl:text>)</xsl:text>
            </span>
        </xsl:if>
    </xsl:element>
</xsl:template>

<!-- The original links to the authors' issue tracker. Reports on the  -->
<!-- Korean text belong to the translation project instead.             -->
<xsl:template name="feedback-link">
  <div class="feedback-link">
    <xsl:element name="a">
      <xsl:attribute name="href">
        <xsl:value-of select="$ko.feedback" />
        <xsl:apply-templates select="." mode="internal-id" />
      </xsl:attribute>
      <xsl:attribute name="class">
        <xsl:text>feedback-link</xsl:text>
      </xsl:attribute>
      <xsl:attribute name="target">_blank</xsl:attribute>
      <xsl:attribute name="rel">noopener</xsl:attribute>
      <xsl:text>번역에 대한 의견이나 오류 제보</xsl:text>
      <span class="feedback-link">(GitHub 계정 필요)</span>
    </xsl:element>
  </div>
</xsl:template>

<!-- No PDF is built for the Korean edition. -->
<xsl:template name="pdf-version"/>

<!-- The cover animation is a MathBox demo built with the original toolchain. -->
<xsl:template name="cover"/>

<!-- MathBox demos are not rebuilt: the frame shows the original site's demo. -->
<xsl:template match="mathbox">
    <xsl:variable name="height">
      <xsl:choose>
        <xsl:when test="@height">
          <xsl:value-of select="@height"/>
        </xsl:when>
        <xsl:otherwise>
          <xsl:text>300px</xsl:text>
        </xsl:otherwise>
      </xsl:choose>
    </xsl:variable>
    <xsl:element name="div">
        <xsl:attribute name="id">
            <xsl:apply-templates select="." mode="internal-id" />
        </xsl:attribute>
        <xsl:attribute name="class">
            <xsl:text>mathbox</xsl:text>
        </xsl:attribute>
        <xsl:attribute name="style">
            <xsl:text>height:</xsl:text>
            <xsl:value-of select="$height" />
            <xsl:text>;</xsl:text>
        </xsl:attribute>
        <xsl:element name="iframe">
          <xsl:attribute name="src">
            <xsl:value-of select="$ila.demos.base"/>
            <xsl:value-of select="@source"/>
          </xsl:attribute>
          <xsl:attribute name="title">대화형 그림(원서 사이트)</xsl:attribute>
          <xsl:attribute name="loading">lazy</xsl:attribute>
        </xsl:element>
        <xsl:element name="div">
          <xsl:attribute name="class">
            <xsl:text>maximizer</xsl:text>
          </xsl:attribute>
          <xsl:text>&#xf25e;</xsl:text>
        </xsl:element>
        <xsl:element name="div">
          <xsl:attribute name="class">
            <xsl:text>minimizer</xsl:text>
          </xsl:attribute>
          <xsl:text>&#xf267;</xsl:text>
        </xsl:element>
    </xsl:element>
</xsl:template>

</xsl:stylesheet>
