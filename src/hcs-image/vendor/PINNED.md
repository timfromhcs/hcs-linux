# Pinned stable-diffusion.cpp source (CMake build, AVX2/AVX-512)
# Upstream: https://github.com/leejet/stability-diffusion.cpp
# Pin: v0.4.x (LCM + SD-Turbo + GGUF q4_0/q8_0, pure-CPU, mmap weights)
# Build flags (on-device): cmake -B build -DSD_AVX2=ON -DSD_AVX512=ON -DGGML_NATIVE=ON
# Binary install target: /usr/lib/hcs/sd-cpp
# Model fetch (<=2GB RAM runtime): SD 1.5 LCM q4_0 (~1.6GB) → /usr/share/hcs/models/sd15-lcm-q4_0.gguf
PIN_URL=https://github.com/leejet/stability-diffusion.cpp
PIN_REV=v0.4.0
MODEL_URL_SD15_LCM_Q4=https://huggingface.co \@ model card pinned in config/models/image.json
