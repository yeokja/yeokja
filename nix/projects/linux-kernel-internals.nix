# linux-kernel-internals: MkDocs Material 사이트 빌드용 Python 환경.
# 원문은 mkdocs 1.6.1·mkdocs-material 9.7.7을 고정하지만(requirements.txt)
# nixpkgs(flake.lock 고정) 버전을 씁니다. 영어 기준 빌드도 같은 셸에서 만들어
# 비교하므로 패치 버전 차이는 검사에 영향을 주지 않습니다. mermaid는 Material이
# 런타임 JS로 그리므로 패키지가 필요 없습니다.
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
      ps.pymdown-extensions
      ps.markdown
    ]))
  ];
  # mkdocs-material 9.7이 MkDocs 2.0 경고 배너를 끄는 데 읽는 변수입니다.
  NO_MKDOCS_2_WARNING = "true";
}
