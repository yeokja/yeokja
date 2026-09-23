# Amortized Cost of a Growable Array

When the array is full, the implementation allocates a new buffer of capacity
$2n$ and copies the existing $n$ elements into it, so a single push can cost
$O(n)$ time.

Starting from capacity $1$, the copies performed during the first $n$ pushes
add up to $1 + 2 + 4 + \dots + 2^{k}$ where $2^{k} < n$, which is less than
$2n$; hence the amortized cost per push is $O(1)$.

If the growth factor is $\alpha$ instead of $2$, the total copy cost becomes
$\frac{\alpha}{\alpha - 1} n$, and the wasted capacity can reach a fraction
$1 - 1/\alpha$ of the buffer.

A factor close to $1$ saves memory but copies often, while a factor such as
$\alpha = 3$ copies rarely but may leave two thirds of the buffer unused.

The potential function $\Phi = 2 \cdot \text{size} - \text{capacity}$ makes the
argument precise: every cheap push raises $\Phi$ by $2$, and every expensive
push spends exactly the potential it has saved.
