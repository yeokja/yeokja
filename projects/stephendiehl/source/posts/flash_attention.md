---
date: "2025-02-11"
tags: ["math", "ai"]
url: "https://www.stephendiehl.com/posts/flash_attention/"
---

# The FlashAttention CUDA Kernel Line by Line

Flash Attention is a memory-efficient algorithm for computing attention in transformers. Let's break down the CUDA implementation block by block. The core innovation of Flash Attention is processing attention in blocks while maintaining numerical stability through careful tracking of maximum values and partial sums. The algorithm achieves memory efficiency by never materializing the full attention matrix while still producing mathematically equivalent results to the standard attention mechanism.

First we need some small macros. The `CEIL_DIV` is used to compute the ceiling of the division of two numbers.

``` c++
#include <cuda.h>
#include <cuda_runtime.h>

#define CEIL_DIV(x, y) ((x) >= 0 ? (((x) + (y) - 1) / (y)) : ((x) / (y)))
```

Now we need the kernel declaration:

``` c++
template <const int Br, const int Bc>
__global__ void flash_attn_kernel(float* Q, float* K, float* V, int N, int d, int Tr, int Tc, float scale, float* l, float* m, float* O)
```

This declares a CUDA kernel with template parameters `Br` and `Bc` which represent the block sizes for rows and columns. The function takes:

- Q, K, V: Query, Key, and Value matrices
- N: Sequence length
- d: Head dimension
- Tr, Tc: Number of blocks in rows and columns
- scale: Scaling factor for attention scores
- l, m: Auxiliary arrays for the numerically stable softmax computation
- O: Output matrix

**Thread and Block Index Setup**

``` c++
int tx = threadIdx.x;
int bx = blockIdx.x;
int by = blockIdx.y;

int qkv_off = (bx * gridDim.y * N * d) + (by * N * d);
int lm_off = (bx * gridDim.y * N) + (by * N);
```

This section retrieves the thread and block indices (tx, bx, by) that identify which thread is executing. It then calculates two important offset values: qkv_off for accessing the Q, K, V matrices and lm_off for accessing the auxiliary arrays l and m. These offsets ensure each block processes the correct portion of the input data.

**Shared Memory Setup**

``` c++
extern __shared__ float smem[];
float* Qi = smem;
float* Kj = Qi + Br * d;
float* Vj = Kj + Bc * d;
float* Sij = Vj + Bc * d;
float* Oi = Sij + Br * Bc;
float* li = Oi + Br * d;
float* li_new = li + Br;
float* mi = li_new + Br;
float* mi_new = mi + Br;
float* mij_dash = mi_new + Br;
```

This section partitions the shared memory into different regions for:

- Qi: Current block of Query matrix
- Kj: Current block of Key matrix
- Vj: Current block of Value matrix
- Sij: Attention scores
- Oi: Output accumulator
- li, li_new, mi, mi_new, mij_dash: Softmax computation variables

**Main Processing Loops**

The kernel uses two nested loops:

1.  Outer loop over Key/Value blocks (`j`)
2.  Inner loop over Query blocks (`i`)

**Key-Value Loading**

``` c++
for (int j = 0; j < Tc; j++) {
    int loads_per_thread = CEIL_DIV(d, Br);
    for (int e = 0; e < loads_per_thread; e++) {
        int idx = e * (Br * Bc) + tx;
        if (idx < Bc * d) {
            int row = idx / d;
            int col = idx % d;

            if (j * Bc + row < N) {
                Kj[row * d + col] = K[qkv_off + (j * Bc + row) * d + col];
                Vj[row * d + col] = V[qkv_off + (j * Bc + row) * d + col];
            }
        }
    }
    __syncthreads();
```

This section loads the current block of K and V matrices into shared memory, with bounds checking to prevent out-of-bounds access.

Let's break down the main computation loop which processes each block of queries:

**Loading Query and Output Data**

``` c++
for (int e = 0; e < loads_per_thread; e++) {
    int idx = e * (Br * Bc) + tx;
    if (idx < Br * d) {
        int row = idx / d;
        int col = idx % d;
        if (i * Br + row < N) {
            Qi[row * d + col] = Q[qkv_off + (i * Br + row) * d + col];
            Oi[row * d + col] = O[qkv_off + (i * Br + row) * d + col];
        }
    }
}
```

This section loads the current block of Query matrix and Output accumulator into shared memory. Each thread may load multiple elements to ensure efficient memory access patterns. The bounds checking ensures we don't access out-of-bounds memory when N isn't perfectly divisible by the block size.

**Thread Position Calculation and Max/Sum Loading**

``` c++
int s_row = tx / Bc;
int s_col = tx % Bc;

if (s_col == 0) {
    mi[s_row] = m[lm_off + (i * Br) + s_row];
    li[s_row] = l[lm_off + (i * Br) + s_row];
}
__syncthreads();
```

Each thread calculates its position in the shared memory block. The first thread in each row (`s_col == 0`) loads the running maximum and sum values needed for the numerically stable softmax computation.

**Computing Attention Scores**

``` c++
float acc = 0.f;
for (int k = 0; k < d; k++)
    acc += Qi[s_row * d + k] * Kj[s_col * d + k];

acc *= scale;
Sij[s_row * Bc + s_col] = acc;
```

This computes the scaled dot product attention scores between the Query and Key matrices. Each thread computes one element of the attention score matrix Sij.

**Numerically Stable Softmax Computation**

``` c++
if (s_col == 0) {
    float row_m = -INFINITY, row_l = 0.f;
    // Find max for numerical stability
    for (int c = 0; c < Bc; c++) {
        float val = Sij[s_row * Bc + c];
        if (val > row_m) {
            row_m = val;
        }
    }
    // Compute exponentials and sum
    for (int c = 0; c < Bc; c++) {
        float exp_val = expf(Sij[s_row * Bc + c] - row_m);
        Sij[s_row * Bc + c] = exp_val;
        row_l += exp_val;
    }

    // Update running max and sum
    mij_dash[s_row] = row_m;
    mi_new[s_row] = max(mi[s_row], row_m);
    li_new[s_row] = expf(mi[s_row] - mi_new[s_row]) * li[s_row] +
                    expf(row_m - mi_new[s_row]) * row_l;
}
```

This implements the numerically stable softmax algorithm. For each row:

1.  Find the maximum value for numerical stability
2.  Compute exponentials normalized by the maximum
3.  Sum the normalized exponentials
4.  Update the running maximum and sum for the online softmax computation

**Output Computation and Update**

``` c++
for (int col = s_col; col < d; col += Bc) {
    float acc = 0.f;
    for (int c = 0; c < Bc; c++)
        acc += Sij[s_row * Bc + c] * Vj[c * d + col];

    int global_row = (i * Br) + s_row;
    if (global_row < N) {
        Oi[s_row * d + col] = (1 / li_new[s_row]) *
            ((li[s_row] * expf(mi[s_row] - mi_new[s_row]) * Oi[s_row * d + col]) +
             (expf(mij_dash[s_row] - mi_new[s_row]) * acc));
        O[qkv_off + global_row * d + col] = Oi[s_row * d + col];
    }
}
```

This final section:

1.  Computes the matrix multiplication between the softmax probabilities and the Value matrix
2.  Updates the output using the online softmax algorithm
3.  Each thread may compute multiple output elements (when d \> Bc)
4.  The reason for the complex update formula is to ensure numerical stability while accumulating partial results

**State Update for Next Iteration**

``` c++
m[lm_off + (i * Br) + s_row] = mi_new[s_row];
l[lm_off + (i * Br) + s_row] = li_new[s_row];
```

Finally, we update the running maximum and sum values in global memory for the next iteration. This is crucial for the online softmax computation across blocks.
