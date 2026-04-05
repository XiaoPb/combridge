#include "gh_protocol_api.h"
#include "gh_rpccore.h"
#include "gh_rpc_functions.h"
#include "gh_data_package.h"
#include "gh_config.h"
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>

struct gh_protocol_handle {
    GhRPCInitialInfo rpc_info;
    gh_protocol_event_fn event_callback;
    gh_protocol_frame_fn frame_callback;
};

static gh_protocol_handle_t* g_protocol_handle = NULL;

static int32_t g_frame_rawdata[GH_FRAME_RAWDATA_MAX_SIZE];
static int32_t g_frame_gs_data[GH_FRAME_GS_DATA_MAX_SIZE];
static int32_t g_frame_flags[GH_FRAME_FLAGS_MAX_SIZE];
static int32_t g_frame_algo_data[GH_FRAME_ALGO_DATA_MAX_SIZE];
static int32_t g_frame_agc_info[GH_FRAME_AGC_INFO_MAX_SIZE];
static int32_t g_frame_agc_info_high[GH_FRAME_AGC_INFO_MAX_SIZE];
static int32_t g_frame_phy_value[GH_FRAME_PHY_VALUE_MAX_SIZE];

static void internal_event_handler(uint8_t event_type, uint8_t* data, uint32_t size)
{
    if (g_protocol_handle && g_protocol_handle->event_callback) {
        g_protocol_handle->event_callback(event_type, data, size);
    }
}

static void internal_frame_handler(uint8_t* data, int size)
{
    if (g_protocol_handle && g_protocol_handle->frame_callback) {
        data_frame_t frame;
        memset(&frame, 0, sizeof(frame));
        
        frame.p_rawdata = g_frame_rawdata;
        frame.p_gs_data = g_frame_gs_data;
        frame.p_flags = g_frame_flags;
        frame.p_algo_data = g_frame_algo_data;
        frame.p_agc_info = g_frame_agc_info;
        frame.p_agc_info_high = g_frame_agc_info_high;
        frame.p_phy_value = g_frame_phy_value;
        
        if (gh_protocol_bytes_to_frame(data, size, &frame) > 0) {
            g_protocol_handle->frame_callback(&frame);
        }
    }
}

gh_protocol_handle_t* gh_protocol_create(const gh_protocol_config_t* config)
{
    if (!config) {
        return NULL;
    }

    gh_protocol_handle_t* handle = (gh_protocol_handle_t*)malloc(sizeof(gh_protocol_handle_t));
    if (!handle) {
        return NULL;
    }

    memset(handle, 0, sizeof(gh_protocol_handle_t));
    handle->rpc_info.lock = config->lock;
    handle->rpc_info.unlock = config->unlock;
    handle->rpc_info.delay = config->delay;
    handle->rpc_info.sendFunction = config->send;
    handle->rpc_info.mode = 0;
    handle->event_callback = config->event_callback;
    handle->frame_callback = config->frame_callback;

    g_protocol_handle = handle;
    GHRPC_init(handle->rpc_info);

    return handle;
}

void gh_protocol_destroy(gh_protocol_handle_t* handle)
{
    if (handle) {
        if (g_protocol_handle == handle) {
            g_protocol_handle = NULL;
        }
        free(handle);
    }
}

int gh_protocol_receive(gh_protocol_handle_t* handle, uint8_t* data, uint32_t size)
{
    if (!handle || !data || size == 0) {
        return -1;
    }

    GHRPC_process(data, (int)size, 0);
    return 0;
}

int gh_protocol_send_raw(gh_protocol_handle_t* handle, const char* key, const char* format, ...)
{
    if (!handle || !key) {
        return -1;
    }

    va_list args;
    va_start(args, format);
    int ret = GHRPC_publish(key, format, args);
    va_end(args);

    return ret;
}

#if GH_PROTOCOL_POLL_EN
void gh_protocol_poll(gh_protocol_handle_t* handle)
{
    (void)handle;
}
#endif

void gh_protocol_get_version(gh_protocol_handle_t* handle, uint8_t ver_type, uint8_t* ver, uint16_t* size)
{
    if (!handle || !ver || !size) {
        return;
    }

    uint8_t ret[256];
    RPCPoint pver = {ver, 0};
    GHRPC_call(ret, "GH3X_GetVersion", "<u8>", ver_type);
    GHRPC_unpack(ret, "<u8*>", &pver);
    *size = (uint16_t)pver.size;
}

void gh_protocol_regs_write(gh_protocol_handle_t* handle, uint16_t* regs, int32_t size)
{
    if (!handle || !regs) {
        return;
    }

    RPCPoint pregs = {regs, (size_t)size};
    GHRPC_send("GH3X_RegsWriteCmd", "<u16*>", &pregs);
}

void gh_protocol_regs_read(gh_protocol_handle_t* handle, uint16_t reg_addr, int32_t read_len, uint16_t* reg_value, int32_t* len)
{
    if (!handle || !reg_value || !len) {
        return;
    }

    uint8_t ret[256];
    RPCPoint pregs = {reg_value, 0};
    GHRPC_call(ret, "GH3X_RegsReadCmd", "<u16><d32>", reg_addr, read_len);
    GHRPC_unpack(ret, "<u16*>", &pregs);
    *len = (int32_t)pregs.size;
}

void gh_protocol_reg_bitfield_write(gh_protocol_handle_t* handle, uint16_t reg_addr, uint8_t lsb, uint8_t msb, uint16_t reg_val)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_RegBitFieldWriteCmd", "<u16><u8><u8><u16>", reg_addr, lsb, msb, reg_val);
}

void gh_protocol_chip_ctrl(gh_protocol_handle_t* handle, uint8_t ctrl_type)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_ChipCtrl", "<u8>", ctrl_type);
}

void gh_protocol_download_config(gh_protocol_handle_t* handle, uint8_t stage)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_DownloadConfig", "<u8>", stage);
}

void gh_protocol_regs_list_write(gh_protocol_handle_t* handle, uint16_t* regs, uint16_t size)
{
    if (!handle || !regs) {
        return;
    }

    RPCPoint pregs = {regs, (size_t)size};
    GHRPC_send("GH3X_RegsListWriteCmd", "<u16*>", &pregs);
}

void gh_protocol_sw_function_cmd(gh_protocol_handle_t* handle, uint32_t target_func_mode, uint8_t ctrl_type)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_SwFunctionCmd", "<u32><u8>", target_func_mode, ctrl_type);
}

void gh_protocol_low_power_cmd(gh_protocol_handle_t* handle, uint32_t target_func_mode, uint8_t ctrl_type)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_LowPowerCmd", "<u32><u8>", target_func_mode, ctrl_type);
}

void gh_protocol_fw_update(gh_protocol_handle_t* handle, uint8_t* src, uint32_t len, uint8_t* ret, uint32_t* ret_len)
{
    if (!handle || !src || !ret || !ret_len) {
        return;
    }

    uint8_t resp[256];
    RPCPoint pdata = {src, (size_t)len};
    RPCPoint pret = {ret, 0};
    GHRPC_call(resp, "GH3X_FwUpdateCmd", "<u8*>", &pdata);
    GHRPC_unpack(resp, "<u8*>", &pret);
    *ret_len = (uint32_t)pret.size;
}

void gh_protocol_regs_bitfield_write(gh_protocol_handle_t* handle, uint16_t* reg_bits, uint16_t size)
{
    if (!handle || !reg_bits) {
        return;
    }

    RPCPoint pregs = {reg_bits, (size_t)size};
    GHRPC_send("GH3X_RegsBitFieldWriteCmd", "<u16*>", &pregs);
}

void gh_protocol_set_work_mode(gh_protocol_handle_t* handle, uint8_t work_mode)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_SetWorkModeCmd", "<u8>", work_mode);
}

void gh_protocol_get_chip_link_status(gh_protocol_handle_t* handle, uint8_t type, int8_t* status, int32_t* len)
{
    if (!handle || !status || !len) {
        return;
    }

    uint8_t ret[256];
    RPCPoint pstatus = {(uint8_t*)status, 0};
    GHRPC_call(ret, "GH3X_GetChipLinkStatus", "<u8>", type);
    GHRPC_unpack(ret, "<u8*>", &pstatus);
    *len = (int32_t)pstatus.size;
}

void gh_protocol_timestamp_set(gh_protocol_handle_t* handle, uint32_t timestamp)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_TimestampSet", "<u32>", timestamp);
}

void gh_protocol_time_set(gh_protocol_handle_t* handle, uint32_t timestamp, int8_t hour_offset)
{
    if (!handle) {
        return;
    }

    GHRPC_send("GH3X_TimeSet", "<u32><u8>", timestamp, hour_offset);
}

int gh_protocol_bytes_to_frame(uint8_t* buffer, int32_t buffer_size, data_frame_t* frame)
{
    if (!buffer || !frame || buffer_size <= 0) {
        return -1;
    }

    return (int)gh_protocol_bytes_to_rawdata(frame, buffer, buffer_size);
}

void DealFifoDataProcess(uint8_t* buf, int size, uint32_t unFifoId)
{
    (void)buf;
    (void)size;
    (void)unFifoId;
}

void DealFrameDataProcess(uint8_t* buf, int size)
{
    internal_frame_handler(buf, size);
}

void ChipEventProcess(uint8_t* buf, int size)
{
    internal_event_handler(buf[0], buf, size);
}
