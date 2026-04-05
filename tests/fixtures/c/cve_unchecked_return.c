// Modeled after CVE-2023-1855 (xgene-hwmon use-after-free from unchecked init)

// File 1: driver.c
int init_hardware(int dev_id) {
    if (dev_id < 0 || dev_id > 16) return -1;
    int ret = configure_registers(dev_id);
    if (ret < 0) return ret;
    return 0;
}

// File 2: main.c
void startup(int device_id) {
    init_hardware(device_id);  // BUG: return value ignored
    start_dma();               // Proceeds even if hardware init failed
}
