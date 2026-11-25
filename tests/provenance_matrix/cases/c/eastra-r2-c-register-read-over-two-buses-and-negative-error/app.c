#include <linux/i2c.h>
#include <linux/io.h>
void observe(int);
void sample(struct i2c_client *client, unsigned char *registers) {
    if (client) {
        int value = i2c_smbus_read_byte_data(client, 0x10);
        if (value < 0) {
            observe(value);
            return;
        }
        observe(value);
    } else {
        int value = readb(registers + 0x10);
        observe(value);
    }
}
