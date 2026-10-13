# v1-surroundings

`v1`의 항목에 번역 요청 문맥 확장을 적용한 파생 세트입니다(만드는 방법은
`../README.md` 5.1절). 항목 파일의 이름과 라이선스, 저작자, 원문 커밋은
[`../v1/MANIFEST.md`](../v1/MANIFEST.md)와 같습니다. `judge-sample.jsonl`,
`truncation.toml`, `glossaries/`는 `v1`의 사본입니다.

```sh
target/release/yeokja-eval surround --out evals/ko-translation/v1-surroundings
```

원문이나 파서가 바뀌어 세그먼트가 맞는 요청을 찾지 못해 `v1` 요청 그대로 둔 항목:

- `rustc-dev-guide/upstream/src/rustdoc-internals.md#4`
- `synthetic/src/tex/hashing.tex#1`
- `synthetic/src/tex/lattices.tex#1`
