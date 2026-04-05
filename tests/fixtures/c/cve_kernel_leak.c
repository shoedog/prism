// Modeled after CVE-2022-45884 (dvb_register_device memleak), CVE-2022-45887 (ttusb)

struct platform_device;
struct resource { unsigned long start; };

struct my_dev { void *regs; int irq; };

int probe_device(struct platform_device *pdev, struct resource *res) {
    struct my_dev *dev = kzalloc(sizeof(struct my_dev), 0);
    if (!dev) return -1;

    dev->regs = ioremap(res->start, 4096);
    if (!dev->regs) goto err_free;

    dev->irq = platform_get_irq(pdev, 0);
    if (dev->irq < 0) goto err_unmap;

    platform_set_drvdata(pdev, dev);
    return 0;  // BUG: success path doesn't register cleanup for module unload

err_unmap:
    iounmap(dev->regs);
err_free:
    kfree(dev);
    return -1;
}
