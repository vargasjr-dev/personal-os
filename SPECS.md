# Hardware Specifications - Personal OS Dev Machine

**Purpose:** Development workstation for assistant-native operating system with on-device LLM capabilities.

**Target Performance:** Run Llama 3.1 70B at 40+ tokens/sec for real-time assistant interactions.

---

## Complete Build List

### CPU
**AMD Ryzen 9 7950X (16-core, 32-thread)**
- Base Clock: 4.5GHz, Boost: 5.7GHz
- 80MB Cache
- AM5 Socket (DDR5, PCIe 5.0)
- TDP: 170W
- **Price: $550**

**Why:** Best multi-threaded performance for compilation and parallel assistant workloads. Zen 4 architecture is excellent for OS development.

---

### GPU
**NVIDIA RTX 4090 24GB GDDR6X**
- 16,384 CUDA cores
- 24GB VRAM (critical for 70B models)
- 1TB/s memory bandwidth
- Tensor cores for AI acceleration
- **Price: $1,800**

**Why:** The single best GPU for on-device LLM inference. 24GB VRAM allows running 70B models with quantization. CUDA ecosystem is mature for AI work.

---

### RAM
**128GB DDR5 (4x32GB kit)**
- Speed: DDR5-6000 or DDR5-5600
- CL30 or better latency
- Matched kit for stability
- **Price: $400**

**Why:** Large models + OS development + virtualization = heavy RAM usage. 128GB ensures comfortable headroom for multiple LLM models loaded simultaneously.

---

### Motherboard
**ASUS X670E Creator or similar**
- Chipset: AMD X670E
- Socket: AM5
- PCIe 5.0 support (x16 for GPU)
- DDR5 support (4 DIMM slots)
- Multiple M.2 slots
- USB 4.0 / Thunderbolt
- **Price: $350**

**Why:** X670E chipset provides maximum expansion and PCIe lanes. Creator series has excellent build quality and connectivity for workstation use.

---

### Storage
**2TB NVMe SSD (PCIe Gen 4)**
- Samsung 990 Pro or WD Black SN850X
- Read: 7,400+ MB/s
- Write: 6,900+ MB/s
- TBW: 1,200TB+ endurance
- **Price: $150**

**Why:** Fast storage for OS development, model loading, and build artifacts. Gen 4 is plenty fast; Gen 5 premium isn't worth it yet.

---

### Power Supply
**1000W 80+ Gold Modular PSU**
- Corsair RM1000x or EVGA SuperNOVA 1000 G6
- Fully modular cables
- 10-year warranty
- **Price: $180**

**Why:** RTX 4090 can spike to 450W+. Ryzen 9 7950X is 170W. Total system under load: ~700W. 1000W provides safe overhead and efficiency sweet spot.

---

### Case
**Fractal Design Torrent (or Meshify 2 XL)**
- Full tower or mid-tower
- Excellent airflow design
- Tool-less design
- Sound dampening
- **Price: $180**

**Why:** RTX 4090 is physically large and runs hot. Torrent has best-in-class airflow. Clean cable management for a workstation build.

---

### CPU Cooler
**Noctua NH-D15 (or NH-D15 chromax.black)**
- Dual tower design
- 2x 140mm fans
- AM5 bracket included
- Ultra quiet operation
- **Price: $110**

**Why:** Handles 7950X thermals easily. Quieter than AIO water cooling. Zero maintenance. Noctua legendary reliability.

---

## Total Cost: **$3,720**

---

## Performance Expectations

### LLM Inference
- **Llama 3.1 70B (4-bit quantized):** 40-50 tokens/sec
- **Llama 3.1 13B:** 120+ tokens/sec
- **Multiple 7B models:** Can run 3-4 simultaneously

### OS Development
- **Kernel compilation:** Sub-10 seconds for incremental builds
- **Full system rebuild:** Under 2 minutes
- **QEMU virtualization:** Near-native performance

### Longevity
- **5+ year relevance** for cutting-edge AI work
- **Upgradeable:** Can add 2nd GPU, more RAM, more storage
- **No vendor lock-in:** Full control over every component

---

## Build Notes

### Assembly Tips
1. Install CPU and RAM on motherboard **before** mounting in case
2. Install motherboard I/O shield first (always forgotten!)
3. Connect all power cables before cable management
4. RTX 4090 is **heavy** - may need GPU support bracket
5. Update BIOS to latest before first boot

### Initial Software Setup
1. Boot from USB with minimal Linux (Arch, Ubuntu Server, or custom kernel)
2. Install Rust toolchain (nightly)
3. Install QEMU for virtualization testing
4. Configure CUDA/cuDNN for LLM inference
5. Begin OS development!

### Alternative Configurations

**Budget (-$800):** RTX 4080 Super instead of 4090
- Saves $800, still excellent for 70B models
- Total: **$2,920**

**Overkill (+$2,000):** Dual RTX 4090s
- Run 405B models or multiple 70B models
- Total: **$5,720**

---

## Why This Over Mac M5?

**Apple Silicon Limitations:**
- ❌ Cannot boot custom OS (locked bootloader)
- ❌ Cannot modify low-level system behavior
- ❌ No CUDA support for AI workloads
- ❌ Not upgradeable

**This Build Advantages:**
- ✅ Total hardware freedom
- ✅ Boots anything (custom kernel, bare metal)
- ✅ 40% faster LLM inference (RTX 4090 vs M5)
- ✅ Upgradeable and future-proof
- ✅ Full Linux/open-source ecosystem

---

## Next Steps

1. **Save $3,720** (or $2,920 for budget build)
2. **Order parts** (use PCPartPicker for current pricing)
3. **Build day** (2-3 hours assembly)
4. **Boot custom kernel** within 24 hours
5. **Begin assistant-native OS development**

---

**The future of computing starts here.** ⚔️

*Updated: 2026-03-10*
