---
date: "2026-10-11"
tags: ["ai", "python", "llms", "compilers", "mlir"]
url: "https://www.stephendiehl.com/posts/mlir_gpu_target/"
---

<img src="https://www.stephendiehl.com/images/mlir_gpu_target.jpeg" width="500" alt="GPU Target" />

# GPT-2 on the GPU

Part 8 built the GPU path by hand: kernels in the `gpu` dialect, NVVM lowering, PTX, and the driver calls that allocate memory and launch a grid. Parts 9 through 13 built a tensor compiler whose output ran on the CPU. This chapter joins the two. The same traced tensor programs, with the same formulas and the same reference checks, run on an NVIDIA GPU.

There are three pieces of work. The parallel loops that remain after bufferization have to become kernels. Matrix multiplication needs a schedule that uses shared memory, since a naive thread-per-output mapping wastes the hardware. And the boundary between host arrays and device memory has to be decided. All of it lives in `tinymlir/gpu.py`, a module of about two hundred and fifty lines that the CPU path never calls into.

The chapter ends with a second way to compile the model. Rather than one function per sublayer, the whole NumPy reference forward pass is traced into a single module, and that module is handed to either target.

You need Linux on x86_64, an NVIDIA driver and a card of compute capability sm_75 or newer. A Tesla T4 on Google Colab is enough; the repository includes a [notebook](https://colab.research.google.com/github/sdiehl/tiny-mlir/blob/main/Colab.ipynb) that installs the CUDA build of the bindings and runs everything below. Nothing else in the series requires a GPU.

## Where the Targets Diverge

Recall the shape of the pipeline from Part 12. Fusion runs on tensors. Bufferization assigns storage and inserts deallocations. The result is a module of `linalg` operations over memrefs, and that module is what each target receives:

``` python
PassManager.parse(BUFFERIZATION_PIPELINE).run(lowered.operation)
if target == GPU:
    gpu.lower(lowered)
else:
    blas.rewrite(lowered)
    PassManager.parse(LOWERING_PIPELINE).run(lowered.operation)
```

The GPU side has the same two-step structure as the CPU side, a substitution followed by a pipeline:

``` python
def outline(module):
    tiled_matmuls(module)
    PassManager.parse(KERNEL_PIPELINE).run(module.operation)
    managed_allocations(module)


def lower(module):
    outline(module)
    PassManager.parse(llvm_pipeline()).run(module.operation)
```

`outline` is the part that is independent of hardware. It runs in the test suite on every platform, including machines with no GPU, and asserts that every loop in the module became a kernel launch. `lower` continues to PTX and needs the CUDA build of MLIR, which exists only for Linux.

The order within `outline` matters. The matmul substitution has to come before the loop passes, otherwise the matmuls would be converted to parallel loops and nothing would be left to substitute. And both have to come after bufferization and buffer deallocation. The hand-written kernel contains `gpu.barrier`, whose memory effects the deallocation analysis can't see through. By the time the kernel is inserted, storage has already been assigned and released.

## Loops to Kernels

The second step is a pipeline of existing MLIR passes:

``` python
KERNEL_PIPELINE = (
    "builtin.module("
    "convert-linalg-to-parallel-loops,"
    "func.func(scf-parallel-loop-tiling{parallel-loop-tile-sizes=16,16 "
    "no-min-max-bounds=true}),"
    "func.func(gpu-map-parallel-loops),"
    "convert-parallel-loops-to-gpu,"
    "gpu-kernel-outlining,canonicalize)"
)
```

Each `linalg.generic` becomes an `scf.parallel` loop nest over its parallel iterators, with any reduction iterators as ordinary sequential `scf.for` loops inside. A softmax row reduction is therefore a parallel loop over rows containing a serial loop along each row. One thread handles one row. That is the right mapping for GPT-2's shapes, where the reduced dimension is at most a few thousand elements, and it is the only reduction strategy this backend has.

Tiling splits each parallel loop into an outer loop over tiles of sixteen and an inner loop within the tile. The mapping pass annotates the outer loops as block dimensions and the inner loops as thread dimensions. Conversion turns the annotated nest into a `gpu.launch`, and outlining moves its body into a `gpu.func` inside a `gpu.module`, replacing it with `gpu.launch_func`. For the GELU of a `(64, 256)` matrix, the host function after this pipeline is:

```
%memref = gpu.alloc host_shared () : memref<64x256xf64>
gpu.launch_func @kernel_kernel::@kernel_kernel
    blocks in (%c4, %c16, %c1) threads in (%c16, %c16, %c1)
    args(%c16 : index, %c0 : index, %c16 : index, %c1 : index, %c0 : index,
         %arg0 : memref<64x256xf32>, %cst_2 : f32, %cst_1 : f32,
         %cst_0 : f64, %cst : f64, %cst_3 : f32, %memref : memref<64x256xf64>)
memref.copy %memref, %arg1 : memref<64x256xf64> to memref<64x256xf64>
```

A grid of four by sixteen blocks of sixteen by sixteen threads covers the matrix. The kernel's body is the fused GELU region from Part 10 with `gpu.block_id` and `gpu.thread_id` in place of loop induction variables:

```
gpu.func @kernel_kernel(..., %arg5: memref<64x256xf32>, ..., %arg11: memref<64x256xf64>) kernel {
  %block_id_x = gpu.block_id x
  %thread_id_x = gpu.thread_id x
  ...
  %6 = memref.load %arg5[%4, %5] : memref<64x256xf32>
  %7 = math.powf %6, %arg6 : f32
  ...
  %12 = math.tanh %11 : f64
  ...
  memref.store %16, %arg11[%4, %5] : memref<64x256xf64>
  gpu.return
}
```

The `no-min-max-bounds` option tells the tiling pass not to clamp the inner loop with `min` expressions when a dimension is not a multiple of sixteen, and to use a conditional in the body instead. The clamped form produces loop bounds that the GPU conversion can't map to a fixed block size.

The test suite checks the outcome structurally. For GELU, softmax, layer normalization, matmul, gather and attention, the host function after `outline` must contain `gpu.launch_func` and no `scf`, `linalg` or `memref.alloc` operations. Everything that iterated now runs on the device.

## A Tiled Matrix Multiplication

Running `linalg.matmul` through the same pipeline would work, and would be slow. It becomes a parallel loop over the output with a serial loop over $K$ inside, so each thread reads a full row of $A$ and a full column of $B$ from global memory. Adjacent threads in a block read overlapping data and nothing is shared. Part 8 introduced shared memory precisely for this case.

The backend therefore replaces every floating-point `linalg.matmul` and `linalg.batch_matmul` with a call to a kernel written by hand. The kernel is a textual MLIR template, formatted for the operand rank and element type, parsed with `ir.Module.parse` and moved into the module. Omitting the batch handling and some of the bounds checks, its structure is:

```
func.func private @tiled_matmul_2d_f32(%a: memref<?x?xf32, ...>, %b: ..., %out: ...) {
  %m = memref.dim %a, %i0 : ...
  %k = memref.dim %a, %i1 : ...
  %n = memref.dim %b, %i1 : ...
  %gx = arith.ceildivui %n, %tile : index
  %gy = arith.ceildivui %m, %tile : index
  gpu.launch blocks(%bx, %by, %bz) in (%sx = %gx, %sy = %gy, %sz = %i1)
             threads(%tx, %ty, %tz) in (%ux = %tile, %uy = %tile, %uz = %i1)
             workgroup(%sa : memref<16x16xf32, #gpu.address_space<workgroup>>,
                       %sb : memref<16x16xf32, #gpu.address_space<workgroup>>) {
    %row = arith.addi (arith.muli %by, %tile), %ty
    %col = arith.addi (arith.muli %bx, %tile), %tx
    %sum = scf.for %k0 = %i0 to %k step %tile iter_args(%acc = %zero) -> (f32) {
      // each thread stages one element of the A tile and one of the B tile
      memref.store (load %a[%row, %k0 + %tx] or zero), %sa[%ty, %tx]
      memref.store (load %b[%k0 + %ty, %col] or zero), %sb[%ty, %tx]
      gpu.barrier
      %part = scf.for %kk = %i0 to %tile step %i1 iter_args(%s = %acc) -> (f32) {
        %x = memref.load %sa[%ty, %kk]
        %y = memref.load %sb[%kk, %tx]
        scf.yield (%s + %x * %y)
      }
      gpu.barrier
      scf.yield %part : f32
    }
    scf.if (%row < %m and %col < %n) {
      %old = memref.load %out[%row, %col]
      memref.store (%old + %sum), %out[%row, %col]
    }
    gpu.terminator
  }
  return
}
```

Each block owns a sixteen by sixteen tile of the output. It walks the reduction dimension in steps of sixteen. At every step each of the block's two hundred and fifty-six threads loads one element of the current $A$ tile and one of the current $B$ tile into the two `workgroup` buffers, out-of-range positions reading as zero. A barrier makes the tile visible to every thread. Each thread then accumulates sixteen products from shared memory, and a second barrier keeps the next step's loads from overwriting a tile still being read. Every global element is loaded once per block instead of once per thread, a factor of sixteen fewer global loads.

Two details connect the kernel to the surrounding program. The final store adds to the output rather than overwriting it, because `linalg.matmul` accumulates into its destination; the builder's `linalg.fill` with zero runs first as its own small kernel. And the kernel takes memrefs with fully dynamic sizes, strides and offset. The substitution inserts a `memref.cast` for each operand, so a single kernel per rank and element type serves every shape in the model, and strided views from slicing and transposition work without copies:

```
%cast = memref.cast %arg0 : memref<64x256xf32> to memref<?x?xf32, strided<[?, ?], offset: ?>>
%cast_0 = memref.cast %arg1 : memref<256x32xf32> to memref<?x?xf32, strided<[?, ?], offset: ?>>
%cast_1 = memref.cast %memref : memref<64x32xf32> to memref<?x?xf32, strided<[?, ?], offset: ?>>
call @tiled_matmul_2d_f32(%cast, %cast_0, %cast_1)
```

The batched variant adds a third grid dimension indexed by the batch, which attention's per-head products use.

This is the classic shared-memory matmul from the CUDA programming guide, no more. There is no register tiling, no vectorized or asynchronous loads, no tensor cores, and the tile size is fixed. cuBLAS would be several times faster on these shapes. The point is that the kernel is sixty lines of readable IR, inserted at the one place in the pipeline where a matrix multiplication is still a matrix multiplication, and it is enough to run the model.

## Managed Memory

Part 8 moved data with explicit copies to and from the device. This backend avoids that bookkeeping with CUDA managed memory, which both the host and the device address through the same pointer. The compiler-side change is small. Every `memref.alloc` left by bufferization becomes a `gpu.alloc` with the `host_shared` attribute, and every `memref.dealloc` becomes the asynchronous `gpu.dealloc` bracketed by `gpu.wait`, which is the form the runtime library implements:

```
%memref = gpu.alloc host_shared () : memref<64x32xf32>
...
%0 = gpu.wait async
%1 = gpu.dealloc async [%0] %memref : memref<64x32xf32>
gpu.wait [%1]
```

The runtime side is a Python class wrapping the same allocator. It calls `mgpuMemAlloc` from MLIR's CUDA runtime library with the host-shared flag and exposes the pointer through the NumPy array interface:

``` python
class Managed:
    def __init__(self, shape, dtype):
        self.pointer = runtime().mgpuMemAlloc(size, None, True)
        self.__array_interface__ = {
            "shape": tuple(shape), "typestr": dtype.str,
            "data": (self.pointer, False), "version": 3,
        }
```

`np.asarray` on such an object is an ordinary NumPy array backed by managed memory. The JIT's call path places each input in managed memory, skipping arrays that already live there, and allocates the output there too. Since the output of one compiled sublayer is the input of the next, the residual stream never leaves the device during a forward pass. The model places its weights in managed memory once at load time.

The cost is that the driver, not the program, decides when pages migrate. A first touch from the host after a kernel writes faults the page back. For this model, where the host reads one row of logits per step, that is the right trade. A latency-sensitive backend would want explicit transfers and streams, and the `gpu` dialect's async tokens are how it would express them.

The execution engine loads the CUDA runtime library alongside the usual runner utilities. The PTX for each `gpu.module` is embedded in the host module as a `gpu.binary`, and a global constructor loads it into the driver when the engine initializes. Launching a kernel from the host function is then a call into the same runtime library that Part 8 called by hand.

## From NVVM to PTX

The last pipeline carries the outlined module to LLVM on both sides of the launch boundary:

```
builtin.module(
  expand-strided-metadata,lower-affine,convert-scf-to-cf,
  nvvm-attach-target{chip=sm_75 O=3},
  gpu.module(convert-gpu-to-nvvm),
  gpu-to-llvm,finalize-memref-to-llvm,
  convert-arith-to-llvm,convert-func-to-llvm,convert-cf-to-llvm,convert-index-to-llvm,
  reconcile-unrealized-casts,
  gpu-module-to-binary{format=isa toolkit=<path to nvidia-cuda-nvcc>})
```

The passes are the ones from Part 8. The target attribute names the chip. `convert-gpu-to-nvvm` rewrites the body of each kernel into NVVM intrinsics for thread indices, barriers and shared memory. `gpu-to-llvm` rewrites the host side, turning allocations, launches and waits into calls into the runtime library. `gpu-module-to-binary` serializes each kernel module to PTX, with `format=isa` keeping the result as text rather than assembling it to SASS, which the driver does at load time for whatever card is present.

The `toolkit` option points the serializer at libdevice. `math.tanh` and `math.exp` inside a kernel aren't single PTX instructions; they link against NVIDIA's bitcode library of math routines. The project gets `libdevice` and `ptxas` from the `nvidia-cuda-nvcc` package on PyPI, so no system CUDA installation is needed, and finds them by importing the package and asking where it lives.

Having `ptxas` available means the PTX can be checked without a GPU. A test lowers each of the six cases above, extracts every `gpu.binary`, and assembles it for sm_75. Continuous integration runs this on an ordinary Linux runner. Malformed PTX from a bad conversion or a missing libdevice link fails the build long before anyone has hardware in hand. The test that compares GPU results with CPU results for the same six functions runs only when a device is present.

## Running It

Pass `--gpu` to the example or to the verification script:

```
uv run python example.py "Alan Turing theorized that computers would one day become" --tokens 10 --gpu
uv run python verify.py --gpu
```

Nothing in `model.py` changed for this. The functions `embed`, `attend`, `feedforward` and `project` are the ones from Part 13, and the target is a keyword passed through to the JIT. On a Colab T4, with compilation included, the ten-token generation took about half a minute and produced the same completion as the CPU and the NumPy reference. The verification script compared embeddings, every block's residuals, logits and token IDs at all ten steps. All ten tokens matched, with a largest absolute difference across the captured arrays of about $1.1\times10^{-5}$, against about $8\times10^{-6}$ on the CPU. The difference is the accumulation order: the tiled kernel sums sixteen products at a time in a fixed order, and the BLAS does something else.

That number is the whole claim. The GPU target produces the reference's required output and agrees with its intermediate values to within tolerance on one card. It isn't fast by the standards of a production runtime.

## The Whole Model as One Kernel

The per-sublayer design compiles four small functions and calls them in a Python loop over blocks. That was a deliberate choice for the series: each compiled function is small enough to print and read. The compiler itself doesn't require it. With the array protocols from Part 9 and the tree-structured arguments from Part 12, a traced function can take the entire parameter record and contain the entire forward pass.

The `fused.py` module does exactly that with the NumPy reference. Its `gpt2` function is the one from `tiny-gpt2`, pasted in and decorated:

``` python
@jit
def gpt2(inputs, params, *, n_head):
    x = params.words[inputs] + params.positions[range(len(inputs))]
    seq_len, embedding_dim = x.shape
    head_size = embedding_dim // n_head

    for block in params.blocks:
        ln1_mean = np.mean(x, axis=-1, keepdims=True)
        ln1_variance = np.var(x, axis=-1, keepdims=True)
        ln1_normalized = (x - ln1_mean) / np.sqrt(ln1_variance + 1e-5)
        ln1_output = block.attention_norm.weight * ln1_normalized + block.attention_norm.bias

        qkv_proj = ln1_output @ block.attention_in.weight + block.attention_in.bias
        q_proj, k_proj, v_proj = np.split(qkv_proj, 3, axis=-1)
        ...
        causal_mask = np.triu(np.ones((seq_len, seq_len), dtype=x.dtype) * -np.inf, k=1)
        attention_scores = attention_scores + causal_mask
        ...
```

Every NumPy call dispatches to our symbolic value. `params` is the model's named tuple of weights, flattened to arrays by the JIT and reassembled inside the trace. The `np.triu` mask is evaluated by NumPy itself, because none of its arguments is symbolic; the resulting array is captured as a constant and lifted to a trailing kernel argument. There are twelve of them, one per block, each a `(T, T)` matrix. This implementation masks with `-np.inf` where Part 11's mask used $-10^{10}$; after the softmax both give the future positions exactly zero weight.

Tracing produces one graph for the twelve blocks. Before optimization the module holds six hundred and fifty-two `linalg.generic` operations; elementwise fusion reduces them to two hundred and nine. The forty-nine `linalg.matmul` and twenty-four `linalg.batch_matmul` operations pass through untouched, which is what the `tensor.reshape` decision in Part 11 was protecting. On the CPU every one of them becomes a GEMM call. On the GPU every one becomes a launch of the tiled kernel.

A separate specialization per sequence length would mean a compilation per generated token, and compiling this module is not cheap. The fused mode pads the token sequence to the next power of two and reads back the row of logits for the last real token:

``` python
def logits(ids, weights, config, target):
    padded = np.zeros(min(1 << (len(ids) - 1).bit_length(), config.context), np.int32)
    padded[: len(ids)] = ids
    output = gpt2(padded, weights, n_head=config.heads, target=target)
    return output[len(ids) - 1 : len(ids)]
```

The causal mask guarantees that positions after the last real token don't influence it, so padding changes nothing in the row we read. A ten-token prompt generating ten tokens needs lengths sixteen and thirty-two, two compilations instead of ten.

```
uv run python example.py "Alan Turing theorized that computers would one day become" --tokens 10 --fused
uv run python verify.py --fused
uv run python example.py "..." --tokens 10 --gpu --fused
```

All three produce the same sentence. In the fused mode the forward pass is one function with no intermediate values to capture, so the verification script compares only the final logits and the selected tokens. On a laptop the fused run took roughly three times as long as the per-sublayer run, nearly all of it LLVM compiling two large modules. On the T4 the two modes took about the same time.

What the fused mode demonstrates is a property of the compiler rather than a performance result. The program that was compiled is the reference implementation, unmodified. The series spent four chapters building a frontend with its own operator set so that each piece could be examined in isolation. The same frontend, given an existing NumPy program, traces it whole.

## What the Structure Bought

Look back at what didn't change to add the GPU target. The tensor language, the builder, the optimization pipeline, bufferization and every formula in `ops.py` and `model.py` are the ones from the CPU chapters. The target is a keyword argument. The additions are a loop-to-kernel pipeline assembled from existing passes, one hand-written kernel, and a hundred lines of memory plumbing.

That was possible because the pipeline preserved structure for as long as it could. Matrix multiplication stayed a named operation through fusion and bufferization, so each target could substitute an implementation. Parallel iterators stayed parallel, so a pass could map them to threads. Allocations stayed explicit, so they could be redirected to managed memory. Each decision deferred in the earlier chapters became a seam here.

The limits are equally visible. There is one reduction strategy, one tile size, one element type for the kernel, no key/value cache and no attention schedule that avoids materializing scores. A production compiler would decide each of these per shape and per device. This one makes them readable, and checks the result against the reference on real hardware.

## External Resources

- [MLIR GPU dialect](https://mlir.llvm.org/docs/Dialects/GPU/)
- [MLIR NVVM dialect](https://mlir.llvm.org/docs/Dialects/NVVMDialect/)
- [CUDA Programming Guide: Shared Memory](https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html#shared-memory)
- [CUDA Unified Memory](https://developer.nvidia.com/blog/unified-memory-cuda-beginners/)
- [tiny-mlir Colab notebook](https://colab.research.google.com/github/sdiehl/tiny-mlir/blob/main/Colab.ipynb)
- [MLIR Python wheels with CUDA](https://github.com/makslevental/mlir-wheels)

[Previous: Part 13 - Running GPT-2](mlir_gpt2.md) · [Series contents](mlir_introduction.md)
