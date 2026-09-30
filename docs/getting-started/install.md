# Installing HCS Linux

HCS Linux features an automated graphic installation experience powered by Calamares.

## Step-by-Step Installation

1. Boot your machine using the HCS Linux Live USB media.
2. Select **HCS Live Desktop** from the bootloader menu.
3. Launch the **Install HCS Linux** shortcut on the desktop or dock.
4. Select your preferred Language, Keyboard Layout, and Timezone.
5. In the **Disk Partitioning** step, choose between:
   - Erase Disk (Automated full-disk installation)
   - Encrypted LUKS2 LVM Installation (Recommended for maximum data security)
   - Manual Partitioning
6. Select your target **System Profile**:
   - `Personal / Edge` (Default, optimized for <= 8GB RAM)
   - `Developer Mode` (Installs native compilers, Git, Rust, Python tools)
   - `Security Lab` (Includes authorized network analysis and audit tools)
7. Confirm summary and click **Install**.
8. Reboot into your newly installed HCS Linux environment.
