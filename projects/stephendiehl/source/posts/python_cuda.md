---
date: "2022-11-18"
tags: []
url: "https://www.stephendiehl.com/posts/python_cuda/"
---

# Linking CUDA Kernels into Python

Gaussian blur is a common image processing operation that smooths images using a Gaussian kernel. While it's straightforward to implement on CPU, it can be quite slow for large images. Let's explore how to implement this efficiently using CUDA and then connect it to Python for easy use.

First, install the CUDA toolkit which includes the NVIDIA CUDA compiler (nvcc):

``` bash
# Add NVIDIA package repositories
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
sudo dpkg -i cuda-keyring_1.0-1_all.deb
sudo apt-get update
sudo apt-get install build-essential

# Install CUDA toolkit
sudo apt-get -y install cuda-toolkit-12-x
```

Verify the installation:

``` bash
nvcc --version
```

Create a Python virtual environment and install the required packages:

``` bash
pip install numpy matplotlib pillow pycuda
```

To verify everything is working, run this simple test:

``` python
import pycuda.autoinit
import pycuda.driver as cuda
print(f"CUDA device name: {cuda.Device(0).name()}")
```

If you see your GPU's name printed without errors, you're ready to go. First, let's look at implementing the blur operation in CUDA C. The core of our implementation is a kernel that processes each pixel in parallel:

``` cpp
__global__ void gaussianBlur(
    const unsigned char* input,
    unsigned char* output,
    int width,
    int height,
    float sigma
) {
    const int x = blockIdx.x * blockDim.x + threadIdx.x;
    const int y = blockIdx.y * blockDim.y + threadIdx.y;

    if (x >= width || y >= height) return;

    const int kernel_size = 2 * (int)(3.0f * sigma) + 1;
    const int radius = kernel_size / 2;

    float sum = 0.0f;
    float weightSum = 0.0f;

    for (int ky = -radius; ky <= radius; ky++) {
        for (int kx = -radius; kx <= radius; kx++) {
            const int px = min(max(x + kx, 0), width - 1);
            const int py = min(max(y + ky, 0), height - 1);

            const float distance = (kx * kx + ky * ky);
            const float weight = expf(-distance / (2.0f * sigma * sigma));

            sum += input[py * width + px] * weight;
            weightSum += weight;
        }
    }

    output[y * width + x] = (unsigned char)(sum / weightSum);
}
```

Save this code in a file called `gaussian_blur.cu`.

The kernel works by having each CUDA thread process one pixel in the output image. For each pixel, we calculate a weighted average of the surrounding pixels using a Gaussian distribution. The size of the kernel is determined by sigma - we use a radius of $3\sigma$ to capture most of the Gaussian's significant values.

Each thread first calculates its position in the image using the block and thread indices. We then iterate over a square neighborhood around the pixel, computing Gaussian weights based on the distance from the center pixel. The weights follow the 2D Gaussian function $\exp(-\frac{r^2}{2\sigma^2})$, where \\(r)) is the distance from the center.

To handle image boundaries correctly, we clamp the pixel coordinates using min and max operations. This effectively extends the edge pixels when we would otherwise read outside the image bounds. While the CUDA implementation is efficient, it's more convenient to use it from Python. We can use PyCUDA to compile and run our CUDA kernel from Python code:

``` python
import numpy as np
import pycuda.autoinit
import pycuda.driver as cuda
from pycuda.compiler import SourceModule

def gaussian_blur(image, sigma=1.5):
    # Ensure image is grayscale and float32
    if len(image.shape) > 2:
        image = np.mean(image, axis=2)
    image = image.astype(np.float32)

    # CUDA kernel as a string
    cuda_code = open("gaussian_blur.cu").read()

    # Compile the kernel
    mod = SourceModule(cuda_code)
    gaussian_blur_kernel = mod.get_function("gaussianBlur")

    # Prepare input and output arrays
    height, width = image.shape
    output = np.empty_like(image)

    # Copy input to device
    d_input = cuda.mem_alloc(image.nbytes)
    d_output = cuda.mem_alloc(output.nbytes)
    cuda.memcpy_htod(d_input, image)

    # Set up grid and block dimensions
    block_size = (16, 16, 1)
    grid_size = (
        (width + block_size[0] - 1) // block_size[0],
        (height + block_size[1] - 1) // block_size[1]
    )

    # Run the kernel
    gaussian_blur_kernel(
        d_input,
        d_output,
        np.int32(width),
        np.int32(height),
        np.float32(sigma),
        block=block_size,
        grid=grid_size
    )

    # Copy result back to host
    cuda.memcpy_dtoh(output, d_output)

    return output
```

The Python wrapper handles all the CUDA setup and memory management. It first ensures the input image is in the right format (grayscale and float32). The CUDA kernel is stored as a string and compiled at runtime using PyCUDA's SourceModule. We then allocate memory on the GPU for both input and output images and copy the input data to the GPU. The kernel is launched with a 16x16 thread block size, and the grid size is calculated to ensure we cover the entire image.

After the kernel runs, we copy the result back to CPU memory and return it. Here's how you might use this function:

``` python
import matplotlib.pyplot as plt
from PIL import Image

# Load and convert image to grayscale numpy array
img = np.array(Image.open('lenna.png').convert('L'))

# Apply blur
blurred = gaussian_blur(img, sigma=2.0)

# Display results
plt.figure(figsize=(12, 6))
plt.subplot(121)
plt.imshow(img, cmap='gray')
plt.title('Original')
plt.subplot(122)
plt.imshow(blurred, cmap='gray')
plt.title('Blurred')
plt.show()
```

This implementation provides a significant speedup over CPU-based implementations, especially for larger images. The parallel nature of CUDA allows us to process many pixels simultaneously, and the efficient memory access patterns in the kernel help maintain good performance. However, there is still overhead from copying memory between the CPU and GPU - the input image must be transferred to GPU memory before processing, and the result must be copied back to CPU memory afterwards. For optimal performance in a real application, you'd want to minimize these memory transfers by keeping data on the GPU as much as possible when doing multiple operations.
