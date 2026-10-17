---
date: "2024-11-20"
tags: ["cuda", "ai", "hardware", "linux"]
url: "https://www.stephendiehl.com/posts/setup_gh200_tutorial/"
---

# Setting up a Nvidia GH200 for Development

Setting up an Nvidia GH200 from scratch is a bit fickle. I eventually got it working with CUDA 12.4 and FlashAttention-2, but it took a bit of trial and error. Here's what I did:

The GH200 performs optimally with a 64K kernel page size. Nvidia provides specialized kernel packages for Ubuntu systems. First, remove the existing kernel packages and install the Nvidia 64K kernel:

``` bash
sudo DEBIAN_FRONTEND=noninteractive apt purge linux-image-$(uname -r) \
    linux-headers-$(uname -r) linux-modules-$(uname -r) -y
sudo apt update
sudo apt install linux-nvidia-64k-hwe-22.04 -y
sudo reboot now
```

After the system reboots, verify you're using the correct kernel:

``` bash
uname -r
```

It should read:

```
linux-nvidia-64k-hwe-22.04-555.42.06
```

Now let's update the base system:

``` bash
apt-get update
```

The GH200 requires specific NVIDIA drivers and MLNX_OFED driver. Install these packages. Order is extremely important here!

``` bash
apt-get install -y mlnx-fw-updater mlnx-ofed-all
apt-get install -y cuda-drivers-555 nvidia-kernel-open-555 linux-tools-$(uname -r)
apt-get install -y cuda-toolkit nvidia-container-toolkit
```

The NVIDIA persistence daemon needs to be configured to use the persistence mode.

``` bash
sudo mkdir /lib/systemd/system/nvidia-persistenced.service.d
sudo dd status=none of=/lib/systemd/system/nvidia-persistenced.service.d/override.conf << EOF
[Service]
ExecStart=
ExecStart=/usr/bin/nvidia-persistenced --persistence-mode --verbose
[Install]
WantedBy=multi-user.target
EOF
systemctl enable nvidia-persistenced --now
```

Configure CUDA paths by creating a new profile script:

``` bash
echo "# Library Path for Nvidia CUDA" >> /etc/profile.d/cuda.sh
echo export LD_LIBRARY_PATH=/usr/local/cuda/lib64:'$LD_LIBRARY_PATH' >> /etc/profile.d/cuda.sh
echo export PATH=$HOME/.local/bin:/usr/local/cuda/bin:'$PATH' >> /etc/profile.d/cuda.sh
```

For optimal performance, disable the IRQ balance service and configure NUMA settings:

``` bash
systemctl disable irqbalance
echo "kernel.numa_balancing = 0" >> /etc/sysctl.conf
sysctl -p
```

Now let's enable peer memory access by adding the NVIDIA peer memory module to the system:

``` bash
echo "nvidia-peermem" >> /etc/modules-load.d/nvidia-peermem.conf
```

Verify the module is loaded:

``` bash
lsmod | grep nvidia_peermem
```

After completing all configurations, reboot your system:

``` bash
systemctl disable first-boot.service
reboot
```

After the system reboots, verify your installation by running:

``` bash
nvidia-smi
```

This command should display information about your GH200, including the GPU model, driver version, and current utilization.

``` bash
+-----------------------------------------------------------------------------------------+
| NVIDIA-SMI 555.42.06              Driver Version: 555.42.06      CUDA Version: 12.4     |
|-----------------------------------------+------------------------+----------------------+
| GPU  Name                 Persistence-M | Bus-Id          Disp.A | Volatile Uncorr. ECC |
| Fan  Temp   Perf          Pwr:Usage/Cap |           Memory-Usage | GPU-Util  Compute M. |
|                                         |                        |               MIG M. |
|=========================================+========================+======================|
|   0  NVIDIA GH200 480GB             On  |   00000009:01:00.0 Off |                    0 |
| N/A   33C    P0             76W /  900W |       1MiB /  97871MiB |      0%      Default |
|                                         |                        |             Disabled |
+-----------------------------------------+------------------------+----------------------+

+-----------------------------------------------------------------------------------------+
| Processes:                                                                              |
|  GPU   GI   CI        PID   Type   Process name                              GPU Memory |
|        ID   ID                                                               Usage      |
|=========================================================================================|
|  No running processes found                                                             |
+-----------------------------------------------------------------------------------------+
```

You can inspect the system topology to understand how data travels between GPUs and networking components:

``` bash
nvidia-smi topo -m
```

This command shows the connectivity matrix between GPUs and other PCIe devices. For a GH200, you should see something like:

```
    GPU0      NIC0    NIC1    CPU Affinity    NUMA Affinity    GPU NUMA ID
GPU0     X     NODE    NODE    0-71            1
NIC0    NODE    X      PIX
NIC1    NODE   PIX     X

Legend:
X    = Self
SYS  = Connection traversing PCIe as well as the SMB interconnect between NUMA nodes (e.g., QPI/UPI)
NODE = Connection traversing PCIe as well as the interconnect between PCIe Host Bridges within a NUMA node
PHB  = Connection traversing PCIe as well as a PCIe Host Bridge (typically the CPU)
PXB  = Connection traversing multiple PCIe bridges (without traversing the PCIe Host Bridge)
PIX  = Connection traversing at most a single PCIe bridge
NV#  = Connection traversing a bonded set of # NVLinks

NIC Legend:
NICO: mlx5_0
NIC1: mlx5_1
```

## Setting up vLLM with Flash Attention

First, update Python package management tools:

``` bash
python -m pip install -U pip
python -m pip install -U setuptools wheel
```

Install PyTorch with CUDA 12.4 support:

``` bash
pip install torch --index-url https://download.pytorch.org/whl/cu124
```

Install Flash Attention:

``` bash
pip install flash-attn --no-build-isolation
```

Clone and build vLLM with Flash Attention support:

``` bash
git clone git@github.com:vllm-project/vllm.git
cd vllm
sudo docker build --target build -t vllm_build .
container_id=$(sudo docker create --name vllm_temp vllm_build:latest)
sudo docker cp ${container_id}:/workspace/dist .

# Install vLLM with Flash Attention
pip install vllm-flash-attn
pip install dist/vllm-0.4.2+cu124-cp312-cp312-linux_x86_64.whl
```

Install xformers from source:

``` bash
pip install ninja
# Set TORCH_CUDA_ARCH_LIST if running and building on different GPU types
pip install -v -U git+https://github.com/facebookresearch/xformers.git@main#egg=xformers
```

Set up your environment variables by adding the following to your `~/.bashrc`:

``` bash
export HF_TOKEN=<HF-TOKEN>
export HF_HOME="/home/ubuntu/.cache/huggingface"
export MODEL_REPO=meta-llama/Meta-Llama-3.1-70B-Instruct
```

Apply the changes:

``` bash
source ~/.bashrc
```

Launch the vLLM API server with the following command:

``` bash
vllm serve $MODEL_REPO --dtype auto
```

You can now send requests to the API endpoint at `http://localhost:8000/v1/completions`.

## Setting up sglang with FlashInfer

Alternatively we can serve using SGLang with FlashInfer.

``` bash
# Use the last release branch
git clone -b v0.3.5.post2 https://github.com/sgl-project/sglang.git
cd sglang

pip install -e "python[all]"

pip install flashinfer -i https://flashinfer.ai/whl/cu121/torch2.4/
```

Launch the SGLang server with the following command:

``` bash
python -m sglang.launch_server --model-path meta-llama/Meta-Llama-3.1-70B-Instruct
```

Likewise, you can send requests to the API endpoint at `http://localhost:8000/v1/completions` using the OpenAI API format.
