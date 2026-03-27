#include <stdint.h>

typedef struct { int type; uint8_t data[64]; } ploam_msg_t;
#define RANGING_GRANT   1
#define RANGING_COMPLETE 2
#define ACTIVATE        3
#define DEREGISTRATION  4

enum onu_state { INIT, RANGING, REGISTERED, OPERATIONAL };
static enum onu_state current_state = INIT;

void start_ranging(ploam_msg_t *msg);
void handle_operational_msg(ploam_msg_t *msg);

void handle_ploam_message(ploam_msg_t *msg) {
    switch(current_state) {
        case INIT:
            if (msg->type == RANGING_GRANT) {
                start_ranging(msg);
                current_state = RANGING;
            }
            break;
        case RANGING:
            // BUG: DEREGISTRATION message during RANGING is silently dropped
            if (msg->type == RANGING_COMPLETE) {
                current_state = REGISTERED;
            }
            break;
        case REGISTERED:
            if (msg->type == ACTIVATE) {
                current_state = OPERATIONAL;
            }
            break;
        case OPERATIONAL:
            handle_operational_msg(msg);
            break;
    }
}
