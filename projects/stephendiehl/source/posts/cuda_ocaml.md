---
date: "2024-11-17"
tags: ["ocaml", "cuda", "python"]
url: "https://www.stephendiehl.com/posts/cuda_ocaml/"
---

# Setting up PyTorch for OCaml

Setting up OCaml for modern AI development is a surprisingly niche topic, despite OCaml's strengths in building robust, type-safe systems. This guide focuses specifically on configuring OCaml development environments for NVIDIA's HGX AI platforms with 8x H100 GPUs for doing either continued pre-training or full-tuning of large language models. While a few quant funds have invested heavily in OCaml for their AI infrastructure, there's remarkably little public documentation about setting up OCaml with these dense GPU configurations. We'll walk through a complete setup of OCaml with CUDA and PyTorch on Ubuntu, with specific attention to the unique requirements of these multi-GPU systems that a lot of funds are provisioning these days.

**System Dependencies**

Before starting, ensure you're running Ubuntu. This setup has been tested on Ubuntu and requires root privileges for several installation steps.

``` bash
# Check your Ubuntu version
lsb_release -a
```

First, let's install the basic system dependencies. These packages provide essential development tools and libraries needed for the rest of our setup:

``` bash
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pciutils \
    wget \
    clang \
    libaio-dev \
    python3-pip \
    python3-venv
```

**NVIDIA Driver Installation**

The NVIDIA driver installation varies depending on whether you're running a desktop or headless system:

``` bash
# For desktop systems with X.org or Wayland
sudo apt-get -y install nvidia-driver-550

# For headless systems (servers without display)
sudo apt-get -y install --no-install-recommends nvidia-headless-550

# Install CUDA toolkit
sudo apt-get -y install nvidia-cuda-toolkit
```

This installs the NVIDIA driver version 550 and the CUDA toolkit, which provides the necessary components for GPU acceleration.

**Python Environment Setup**

First, let's install pyenv and configure Python 3.12.4:

``` bash
# Install pyenv dependencies
sudo apt-get install -y make build-essential libssl-dev zlib1g-dev \
libbz2-dev libreadline-dev libsqlite3-dev wget curl llvm \
libncursesw5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev

# Install pyenv
curl https://pyenv.run | bash

# Add pyenv to your shell configuration (~/.bashrc or ~/.zshrc)
echo 'export PYENV_ROOT="$HOME/.pyenv"' >> ~/.bashrc
echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> ~/.bashrc
echo 'eval "$(pyenv init -)"' >> ~/.bashrc

# Reload shell configuration
source ~/.bashrc

# Install Python 3.12.4
pyenv install 3.12.4

# Set Python 3.12.4 as global version
pyenv global 3.12.4

# Verify installation
python --version  # Should output: Python 3.12.4

# Upgrade pip
python -m pip install --upgrade pip

# Install PyTorch with CUDA support
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121

# Install NVIDIA cuDNN
pip install nvidia-cudnn-cu11

# Install Hugging Face libraries
pip install transformers datasets accelerate

# Install scientific computing packages
pip install scipy numpy pandas

# Install optimized attention implementations
pip install flash-attn --no-build-isolation
pip install -v -U git+https://github.com/facebookresearch/xformers.git@main#egg=xformers

# Install GPU monitoring tools
pip install nvitop
```

**DeepSpeed and Megatron-LM Installation**

### DeepSpeed Installation

DeepSpeed provides optimization features for training large models. Let's install it with CUDA operations support:

``` bash
# Basic installation
pip install deepspeed

# OR: Install from source for latest features
git clone https://github.com/microsoft/DeepSpeed.git
cd DeepSpeed
pip install -e .

# OR: Install with pre-compiled operations (recommended)
DS_BUILD_OPS=1 pip install deepspeed --global-option="build_ext" --global-option="-j8"
```

Verify the installation and check which operations are available:

``` bash
ds_report
```

Advanced installation options:

``` bash
# Install specific operations
DS_BUILD_FUSED_ADAM=1 pip install deepspeed    # FusedAdam support
DS_BUILD_FUSED_LAMB=1 pip install deepspeed    # FusedLamb support
DS_BUILD_SPARSE_ATTN=1 pip install deepspeed   # Sparse attention support

# Skip CUDA version check if needed (use with caution)
DS_SKIP_CUDA_CHECK=1 pip install deepspeed
```

### Megatron-LM Installation

Megatron-LM enables training large transformer models at scale. Here's how to set it up:

``` bash
# Install NVIDIA Apex first
git clone https://github.com/NVIDIA/apex
cd apex
pip install -v --disable-pip-version-check --no-cache-dir \
    --global-option="--cpp_ext" \
    --global-option="--cuda_ext" ./
cd ..

# Install Megatron-LM
git clone https://github.com/NVIDIA/Megatron-LM.git
cd Megatron-LM
git checkout core_r0.5.0
pip install --no-use-pep517 -e .
```

**OCaml Setup**

Finally, we'll set up the OCaml environment with PyTorch bindings:

``` bash
# Install OCaml package manager
sudo apt-get install -y opam m4 pkg-config

# Initialize OPAM
opam init --auto-setup --yes
eval $(opam env)

# Create new OCaml environment
opam switch create 4.14.1
eval $(opam env)

# Install OCaml development tools
opam install -y dune merlin ocaml-lsp-server odoc ocamlformat utop

# Install Torch dependencies and bindings
opam install -y ctypes ctypes-foreign conf-pkg-config
opam install -y torch
```

This sets up:

- OPAM (OCaml Package Manager)
- OCaml 4.14.1 environment
- Development tools including:
  - Dune (build system)
  - Merlin (code completion)
  - LSP server (IDE integration)
  - Documentation tools
  - UTop (enhanced REPL)
- PyTorch bindings for OCaml

**Verification**

After installation and system reboot, verify your setup:

``` bash
# Check NVIDIA driver installation
nvidia-smi

# Check GPU interconnect topology
nvidia-smi topo -m

# Start OCaml REPL and test Torch
utop
# In utop:
open Torch;;
```

You should see your GPU listed in the `nvidia-smi` output and be able to import the Torch module in OCaml without errors.

**Optional: Experiment Tracking Setup**

If you plan to track your machine learning experiments, you can set up Weights & Biases alongside Hugging Face:

``` bash
# Install Weights & Biases
pip install wandb

# Log in to Weights & Biases (this will prompt for your API key)
wandb login

# Install and log in to Hugging Face
pip install huggingface_hub
huggingface-cli login
```

You can get your W&B API key from <https://wandb.ai/settings> and your Hugging Face token from <https://huggingface.co/settings/tokens> .

**Example Project: XOR Neural Network**

Let's create the Hello World OCaml project that trains a neural network to learn the XOR function. First, create a new project:

``` bash
# Create project directory
mkdir xor_example
cd xor_example

# Initialize dune project
dune init project xor_example
cd xor_example
```

Now, create a new file `bin/main.ml`:

``` ocaml
open Base
open Torch

(* XOR truth table *)
let input_data =
  Tensor.of_float2 [|
    [|0.; 0.|];
    [|0.; 1.|];
    [|1.; 0.|];
    [|1.; 1.|];
  |]

let target_data =
  Tensor.of_float2 [|
    [|0.|];
    [|1.|];
    [|1.|];
    [|0.|];
  |]

let () =
  (* Create a simple network with one hidden layer *)
  let vs = Var_store.create ~name:"xor" () in
  let hidden = Layer.linear vs ~input_dim:2 4 ~activation:Relu in
  let output = Layer.linear vs ~input_dim:4 1 ~activation:Sigmoid in

  (* Define the model *)
  let model x =
    Layer.forward hidden x
    |> Layer.forward output
  in

  (* Training configuration *)
  let learning_rate = 0.05 in
  let optimizer = Optimizer.adam vs ~learning_rate in

  (* Training loop *)
  for epoch = 1 to 2000 do
    (* Forward pass *)
    let predicted = model input_data in
    let loss = Tensor.mse_loss predicted target_data in

    (* Backward pass *)
    Optimizer.backward_step optimizer ~loss;

    (* Print progress every 100 epochs *)
    if epoch % 100 = 0 then
      Stdio.printf "Epoch %d: Loss = %f\n%!"
        epoch (Tensor.float_value loss)
  done;

  (* Print final results *)
  let final_output = model input_data in
  Stdio.printf "\nFinal predictions:\n";
  Tensor.print final_output
```

Update the `dune` file in the `bin` directory:

``` scheme
(executable
 (name main)
 (libraries base stdio torch))
```

Build and run the project:

``` bash
# Build the project
dune build

# Run the example
dune exec xor_example
```

If that runs correctly, congratuinals you're ready to orchestrate your own training runs with OCaml now.
