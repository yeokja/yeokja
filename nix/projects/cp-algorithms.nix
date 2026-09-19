# cp-algorithms: MkDocs Material 사이트 빌드와 파서 말뭉치 검사용 Python 환경.
# 원문 CI는 버전을 고정하지 않은 pip 설치이므로 nixpkgs(flake.lock 고정) 버전을
# 씁니다. mkdocs-simple-hooks·toggle-sidebar는 nixpkgs에 없어 번역 빌드에서
# 제거하고(scripts/prepare_site.py), git 계열·rss 플러그인은 조립 트리에서
# 동작하지 않으므로 포함하지 않습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    (pkgs.python3.withPackages (ps: [
      ps.mkdocs
      ps.mkdocs-material
      ps.mkdocs-macros
      ps.mkdocs-literate-nav
      ps.pymdown-extensions
      ps.markdown
      ps.pyyaml
    ]))
  ];
  # mkdocs-material 9.7이 MkDocs 2.0 경고 배너를 끄는 데 읽는 변수입니다.
  NO_MKDOCS_2_WARNING = "true";
}
