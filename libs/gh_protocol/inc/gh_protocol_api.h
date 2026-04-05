#ifndef GH_PROTOCOL_API_H
#define GH_PROTOCOL_API_H

#include <stdint.h>
#include <stddef.h>
#include "gh_config.h"
#include "gh_data_package.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct gh_protocol_handle gh_protocol_handle_t;

typedef void (*gh_protocol_lock_fn)(void);
typedef void (*gh_protocol_unlock_fn)(void);
typedef void (*gh_protocol_delay_fn)(void);
typedef void (*gh_protocol_send_fn)(void* data, int size);

/**
 * @brief Event callback function
 * 
 * @param event_type Event type: [RX] input
 * @param data Event data: [RX] input
 * @param size Event data size: [RX] input
 */
typedef void (*gh_protocol_event_fn)(uint8_t event_type, uint8_t* data, uint32_t size);

/**
 * @brief Data frame callback function
 * 
 * @param frame Data frame: [RX] input
 */
typedef void (*gh_protocol_frame_fn)(data_frame_t* frame);

typedef struct {
    gh_protocol_lock_fn lock;
    gh_protocol_unlock_fn unlock;
    gh_protocol_delay_fn delay;
    gh_protocol_send_fn send;
    gh_protocol_event_fn event_callback;
    gh_protocol_frame_fn frame_callback;
} gh_protocol_config_t;

/**
 * @brief Create protocol handle
 * 
 * @param config Protocol configuration: [TX] input
 * @return Protocol handle: [RX] output
 */
gh_protocol_handle_t* gh_protocol_create(const gh_protocol_config_t* config);

/**
 * @brief Destroy protocol handle
 * 
 * @param handle Protocol handle
 */
void gh_protocol_destroy(gh_protocol_handle_t* handle);

/**
 * @brief Receive data
 * 
 * @param handle Protocol handle
 * @param data Data buffer: [RX] input
 * @param size Data buffer size: [RX] input
 * @return Received data length: [RX] output
 */
int gh_protocol_receive(gh_protocol_handle_t* handle, uint8_t* data, uint32_t size);

/**
 * @brief Send raw data
 * 
 * @param handle Protocol handle
 * @param key Command key: [TX] input
 * @param format Format string: [TX] input
 * @param ... Variable arguments: [TX] input
 * @return Sent data length: [RX] output
 */
int gh_protocol_send_raw(gh_protocol_handle_t* handle, const char* key, const char* format, ...);

#if GH_PROTOCOL_POLL_EN
/**
 * @brief Poll data
 * 
 * @param handle Protocol handle
 */
void gh_protocol_poll(gh_protocol_handle_t* handle);
#endif

/**
 * @brief Get protocol version
 * 
 * @param handle Protocol handle
 * @param ver_type Version type: [TX] input
 * @param ver Version number: [RX] output
 * @param size Version number size: [RX] output
 */
void gh_protocol_get_version(gh_protocol_handle_t* handle, uint8_t ver_type, uint8_t* ver, uint16_t* size);

/**
 * @brief Write registers
 * 
 * @param handle Protocol handle
 * @param regs Register value array: [TX] input
 * @param size Register value array size: [TX] input
 */
void gh_protocol_regs_write(gh_protocol_handle_t* handle, uint16_t* regs, int32_t size);

/**
 * @brief Read registers
 * 
 * @param handle Protocol handle
 * @param reg_addr Register address: [TX] input
 * @param read_len Read length: [TX] input
 * @param reg_value Register value array: [RX] output
 * @param len Read length: [RX] output
 */
void gh_protocol_regs_read(gh_protocol_handle_t* handle, uint16_t reg_addr, int32_t read_len, uint16_t* reg_value, int32_t* len);

/**
 * @brief Write register bitfield
 * 
 * @param handle Protocol handle
 * @param reg_addr Register address: [TX] input
 * @param lsb Least significant bit: [TX] input
 * @param msb Most significant bit: [TX] input
 * @param reg_val Register value: [TX] input
 */
void gh_protocol_reg_bitfield_write(gh_protocol_handle_t* handle, uint16_t reg_addr, uint8_t lsb, uint8_t msb, uint16_t reg_val);

/**
 * @brief Control chip
 * 
 * @param handle Protocol handle
 * @param ctrl_type Control type: [TX] input
 */
void gh_protocol_chip_ctrl(gh_protocol_handle_t* handle, uint8_t ctrl_type);

/**
 * @brief Download configuration
 * 
 * @param handle Protocol handle
 * @param stage Download configuration stage: [TX] input (0 or 1)
 */
void gh_protocol_download_config(gh_protocol_handle_t* handle, uint8_t stage);

/**
 * @brief Write register list
 * 
 * @param handle Protocol handle
 * @param regs Register value array: [TX] input
 * @param size Register value array size: [TX] input
 */
void gh_protocol_regs_list_write(gh_protocol_handle_t* handle, uint16_t* regs, uint16_t size);

/**
 * @brief Software function command
 * 
 * @param handle Protocol handle
 * @param target_func_mode Target function mode: [TX] input (see gh_func_fix_id_e)
 * @param ctrl_type Control type: [TX] input (0: stop, 1: start)
 */
void gh_protocol_sw_function_cmd(gh_protocol_handle_t* handle, uint32_t target_func_mode, uint8_t ctrl_type);

/**
 * @brief Low power command
 * 
 * @param handle Protocol handle
 * @param target_func_mode Target function mode: [TX] input
 * @param ctrl_type Control type: [TX] input
 */
void gh_protocol_low_power_cmd(gh_protocol_handle_t* handle, uint32_t target_func_mode, uint8_t ctrl_type);

/**
 * @brief Firmware update
 * 
 * @param handle Protocol handle
 * @param src Firmware data: [TX] input
 * @param len Firmware data length: [TX] input
 * @param ret Firmware update result: [RX] output
 * @param ret_len Firmware update result length: [RX] output
 */
void gh_protocol_fw_update(gh_protocol_handle_t* handle, uint8_t* src, uint32_t len, uint8_t* ret, uint32_t* ret_len);

/**
 * @brief Set working mode
 * 
 * @param handle Protocol handle
 * @param work_mode Working mode: [TX] input
 */
void gh_protocol_set_work_mode(gh_protocol_handle_t* handle, uint8_t work_mode);

/**
 * @brief Get chip link status
 * 
 * @param handle Protocol handle
 * @param type Status type: [TX] input
 * @param status Status value: [RX] output
 * @param len Status value size: [RX] output
 */
void gh_protocol_get_chip_link_status(gh_protocol_handle_t* handle, uint8_t type, int8_t* status, int32_t* len);

/**
 * @brief Set timestamp
 * 
 * @param handle Protocol handle
 * @param timestamp Timestamp: [TX] input
 */
void gh_protocol_timestamp_set(gh_protocol_handle_t* handle, uint32_t timestamp);

/**
 * @brief Set time
 * 
 * @param handle Protocol handle
 * @param timestamp Timestamp: [TX] input
 * @param hour_offset Timezone offset: [TX] input
 */
void gh_protocol_time_set(gh_protocol_handle_t* handle, uint32_t timestamp, int8_t hour_offset);

/**
 * @brief Convert byte array to data frame
 * 
 * @param buffer Byte array: [TX] input
 * @param buffer_size Byte array size: [TX] input
 * @param frame Data frame: [RX] output
 * @return Data frame size: [RX] output
 */
int gh_protocol_bytes_to_frame(uint8_t* buffer, int32_t buffer_size, data_frame_t* frame);

#ifdef __cplusplus
}
#endif

#endif
