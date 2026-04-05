/**
  ****************************************************************************************
 * @file    gh_data_packge.c
 * @author  GOODIX GH Driver Team
 * @brief   gh3036 frame data pack and send
  ****************************************************************************************
  * @attention
  #####Copyright (c) 2024 GOODIX
   All rights reserved.

  Redistribution and use in source and binary forms, with or without
  modification, are permitted provided that the following conditions are met:
  * Redistributions of source code must retain the above copyright
   notice, this list of conditions and the following disclaimer.
  * Redistributions in binary form must reproduce the above copyright
    notice, this list of conditions and the following disclaimer in the
    documentation and/or other materials provided with the distribution.
  * Neither the name of GOODIX nor the names of its contributors may be used
    to endorse or promote products derived from this software without
    specific prior written permission.

  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
  AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
  IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
  ARE DISCLAIMED. IN NO EVENT SHALL COPYRIGHT HOLDERS AND CONTRIBUTORS BE
  LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
  CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
  SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
  INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
  CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
  ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
  POSSIBILITY OF SUCH DAMAGE.

  ****************************************************************************************
  */

/*
 * INCLUDE FILES
 *****************************************************************************************
 */
#include <string.h>
#include "gh_config.h"
#include "gh_hal_log.h"
#include "gh_rpccore.h"
#include "gh_data_package.h"
#include "gh_algo_types.h"

/*
 * DEFINES
 *****************************************************************************************
 */


#if GH_MODULE_PROTOCOL_LOG_EN
#define DEBUG_LOG(...)                              GH_LOG_LVL_DEBUG(__VA_ARGS__)
#define WARNING_LOG(...)                            GH_LOG_LVL_WARNING(__VA_ARGS__)
#define ERROR_LOG(...)                              GH_LOG_LVL_ERROR(__VA_ARGS__)
#else
#define DEBUG_LOG(...)
#define WARNING_LOG(...)
#define ERROR_LOG(...)
#endif

#define GH_BYTES_STREAM_DIFF_EN    1
#define BYTES_BUFFER_SIZE       2048
#define CLEAR_BIT7_MASK       (0x7f)
#define SET_BIT7_MASK         (0x80)
#define SHIFT_BIT_NUM            (7)
#define BIT32_MASK      (0xffffffff)
#define BIT32_NUM               (32)

#define BUFFER_BYTE_THRD       (200)

#define MAX_BIT                 (32)
#define SHIFT_NUM               (31)

#define SLOT_CFG_BIAS           (8)
#define SLOT_CFG_CAP            (13)

#define GH_RAWDATA_MAX          (32)
#define GH_FLAGS_MAX            (32)
#define GH_PHY_VALUE_MAX        (32)
#define GH_TIMESTAMP_MAX         (1)

#if GH_GYRO_EN
    #define GH_GSENSOR_MAX                (GH_ACC_AXIS_NUM + GH_GYRO_AXIS_NUM)
#else
    #define GH_GSENSOR_MAX                (GH_ACC_AXIS_NUM)
#endif

#define GH_ALG_RESULTS_MAX            (32)
#define GH_AGC_INFO_MAX               (32)

#define BYTE_SIZE               (7)
#define COUNT_BYTES(n)          (((n) + BYTE_SIZE - 1) / BYTE_SIZE)

#define GH_FUNC_FIX_IDX_EVK          (8)

/*
 * STRUCT DEFINE
 *****************************************************************************************
 */

/*
 * LOCAL VARIABLE DEFINITIONS
 *****************************************************************************************
 */
uint8_t g_bytes_buffer[BYTES_BUFFER_SIZE] = { 0 };

static int32_t g_last_rawdata[GH_RAWDATA_MAX] = { 0 };
static int32_t g_rawdata_diff[GH_RAWDATA_MAX] = { 0 };
static int32_t g_last_phy_value[GH_PHY_VALUE_MAX] = { 0 };
static int32_t g_phy_value_diff[GH_PHY_VALUE_MAX] = { 0 };
static int32_t g_last_timestamp[GH_TIMESTAMP_MAX] = { 0 };
static int32_t g_last_timestamp_high[GH_TIMESTAMP_MAX] = { 0 };
static int32_t g_timestamp_diff[GH_TIMESTAMP_MAX] = { 0 };
static int32_t g_last_gs_data[GH_GSENSOR_MAX] = { 0 };
static int32_t g_gs_data_diff[GH_GSENSOR_MAX] = { 0 };

static int32_t g_last_flags[GH_FLAGS_MAX] = { 0 };
static int32_t g_last_algo_data[GH_ALG_RESULTS_MAX] = { 0 };
static int32_t g_last_agc_info[GH_AGC_INFO_MAX] = { 0 };
static int32_t g_last_agc_info_high[GH_AGC_INFO_MAX] = { 0 };

static data_frame_bytes_info_t g_bytes_info = {0};

static gh_func_fix_idx_e g_fun_id = GH_FUNC_FIX_IDX_MAX;

static int32_t g_algo_data[GH_ALG_RESULTS_MAX] = { 0 };

static gh_flag_union_t g_flag_u;

/*
 * LOCAL FUNCTION DECLARATION
 *****************************************************************************************
 */
static int32_t gh_protocol_numbits_get(uint32_t num);

static uint32_t zigzag_encode(int32_t data);

static void gh_protocol_bytes_write(uint8_t* buffer, int32_t* pos, int32_t* data, int32_t datasize);

static int32_t gh_protocol_rawdata_to_bytes(data_frame_t* data, uint8_t* buffer, int32_t buffer_size);

static void gh_protocol_lastdata_update(gh_func_frame_t *p_func_frame, data_frame_t *p_frame_info,
                                        data_frame_bytes_info_t *p_bytes_info);

static void gh_protocol_data_diff(gh_func_frame_t* p_func_frame, data_frame_t* p_frame_info,
                                  data_frame_bytes_info_t* p_bytes_info);

static gh_agc_union_t agc_info_combine(gh_func_frame_t *p_func_frame, uint8_t idx);
static uint8_t gh_protocol_algres_update(gh_func_frame_t* p_frame);
static void gh_protocol_publish(data_frame_bytes_info_t* p_bytes_info);

/*
 * LOCAL FUNCTION DEFINITIONS
 *****************************************************************************************
 */
static int32_t gh_protocol_numbits_get(uint32_t num)
{
    int32_t bits = 0;

    while (num)
    {
        num >>= 1;
        bits++;
        if (MAX_BIT == bits)
        {
            break;
        }
    }
    return bits;
}

static uint32_t zigzag_encode(int32_t data)
{
    return (uint32_t)((data >> SHIFT_NUM) ^ (data << 1));
}

static void gh_protocol_bytes_write(uint8_t* buffer, int32_t* pos, int32_t* data, int32_t datasize)
{
    uint32_t num = 0;
    int32_t num_bits = 0;
    int32_t num_bytes = 0;
    uint8_t cur_byte = 0;
    int32_t cnt = datasize;
    int32_t idx = 0;

    while (cnt--)
    {
        num = zigzag_encode(data[idx]);
        num_bits = gh_protocol_numbits_get(num);
        num_bytes = COUNT_BYTES(num_bits);
        if (0 == num_bytes)
        {
            num_bytes = 1;
        }
        while (num_bytes)
        {
            cur_byte = num & CLEAR_BIT7_MASK;
            num >>= SHIFT_BIT_NUM;

            if (num_bytes > 1)
            {
                cur_byte |= SET_BIT7_MASK;
            }
            buffer[*pos] = cur_byte;
            (*pos)++;
            num_bytes--;
        }
        idx++;
    }

    return;
}

static int32_t gh_protocol_rawdata_to_bytes(data_frame_t* data, uint8_t* buffer, int32_t buffer_size)
{
    int32_t pos = 0;

    gh_protocol_bytes_write(buffer, &pos, (int32_t*)&data->pack_header,
                            sizeof(data->pack_header) / sizeof(pack_header_t));

    if (data->pack_header.rawdata_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->rawdata_size, sizeof(data->rawdata_size) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, data->p_rawdata, data->rawdata_size);
    }

    if (data->pack_header.phy_value_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->phy_value_size, sizeof(data->phy_value_size) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, data->p_phy_value, data->phy_value_size);
    }

    if (data->pack_header.gs_data_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->gs_data_size, sizeof(data->gs_data_size) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, data->p_gs_data, data->gs_data_size);
    }

    if (data->pack_header.flags_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->flag_data_bits, sizeof(data->flag_data_bits) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, data->p_flags, data->flag_data_bits);
    }

    if (data->pack_header.alg_data_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->algo_data_bits, sizeof(data->algo_data_bits) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, data->p_algo_data, data->algo_data_bits);
    }

    if (data->pack_header.agc_info_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->agc_info_size, sizeof(data->agc_info_size) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, data->p_agc_info, data->agc_info_size);
        gh_protocol_bytes_write(buffer, &pos, data->p_agc_info_high, data->agc_info_size);
    }

    if (data->pack_header.timestamp_en)
    {
        gh_protocol_bytes_write(buffer, &pos, (int32_t*)(&data->timestamp), sizeof(data->timestamp) / sizeof(int32_t));
        gh_protocol_bytes_write(buffer, &pos, (int32_t*)(&data->timestamp_high),
                                sizeof(data->timestamp_high) / sizeof(int32_t));
    }

    gh_protocol_bytes_write(buffer, &pos, &data->frame_id, sizeof(data->frame_id) / sizeof(int32_t));

    if (data->pack_header.func_id_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->function_id, sizeof(data->function_id) / sizeof(int32_t));
    }

    if (data->pack_header.slot_cfg_en)
    {
        gh_protocol_bytes_write(buffer, &pos, &data->slot_cfg, sizeof(data->slot_cfg) / sizeof(int32_t));
    }

    if (pos > buffer_size)
    {
        // warning
        WARNING_LOG("protocal buffer overflow, buffer_len:%d, max_size:%d", pos, buffer_size);
        return buffer_size;
    }

    return pos;
}

static gh_agc_union_t agc_info_combine(gh_func_frame_t *p_func_frame, uint8_t idx)
{
    gh_agc_union_t agc_info_u;
    gh_agc_upload_t gh_agc_temp;
    gh_agc_temp.gain_code = p_func_frame->p_data[idx].agc_info.gain_code;
    gh_agc_temp.bg_cancel_range = p_func_frame->p_data[idx].agc_info.bg_cancel_range;
    gh_agc_temp.dc_cancel_range = p_func_frame->p_data[idx].agc_info.dc_cancel_range;
    gh_agc_temp.dc_cancel_code = p_func_frame->p_data[idx].agc_info.dc_cancel_code;
    gh_agc_temp.led_drv_fs = p_func_frame->led_drv_fs[0];
    gh_agc_temp.led_drv0 = p_func_frame->p_data[idx].agc_info.led_drv0;
    gh_agc_temp.led_drv1 = p_func_frame->p_data[idx].agc_info.led_drv1;
    gh_agc_temp.led_drv2 = 0;
    gh_agc_temp.led_drv3 = 0;
    agc_info_u.gh_agc_upload = gh_agc_temp;

    return agc_info_u;
}

static void gh_protocol_lastdata_update(gh_func_frame_t *p_func_frame, data_frame_t *p_frame_info,
                                        data_frame_bytes_info_t *p_bytes_info)
{
    if (p_frame_info->pack_header.rawdata_en)
    {
        for (uint8_t i = 0; i < p_frame_info->rawdata_size; i++)
        {
            p_bytes_info->p_last_rawdata[i] = p_func_frame->p_data[i].rawdata;

//            DEBUG_LOG("fun_id:%d, g_last_rawdata[%d]: %d \n", p_func_frame->id,
//                             i, p_bytes_info->p_last_rawdata[i]);
        }
        p_frame_info->p_rawdata = p_bytes_info->p_last_rawdata;
    }

    if (p_frame_info->pack_header.phy_value_en)
    {
        for (uint8_t i = 0; i < p_frame_info->phy_value_size; i++)
        {
            p_bytes_info->p_last_phy_value[i] = p_func_frame->p_data[i].ipd_pa;

            //DEBUG_LOG("g_last_phy_value[%d]: %d \n", i, p_bytes_info->p_last_phy_value[i]);
        }
        p_frame_info->p_phy_value = p_bytes_info->p_last_phy_value;
    }

    if (p_frame_info->pack_header.flags_en)
    {
        for (uint8_t i = 0; i < p_frame_info->flag_data_bits; i++)
        {
            g_flag_u.flag = p_func_frame->p_data[i].flag;
            p_bytes_info->p_last_flags[i] = g_flag_u.value;

            //DEBUG_LOG("g_last_flags[%d]: %d \n", i, p_bytes_info->p_last_flags[i]);
        }
        p_frame_info->p_flags = p_bytes_info->p_last_flags;
    }

    if (p_frame_info->pack_header.alg_data_en)
    {
        for (uint8_t i = 0; i < p_frame_info->algo_data_bits; i++)
        {
//            DEBUG_LOG("fun:%d, res-%d: %d, last_res:%d \r\n", p_func_frame->id,
//                i, g_algo_data[i], p_bytes_info->p_last_algo_data[i]);
            p_bytes_info->p_last_algo_data[i] = g_algo_data[i];

            //DEBUG_LOG("g_last_algo_data[%d]: %d \n", i, p_bytes_info->p_last_algo_data[i]);
        }
        p_frame_info->p_algo_data = p_bytes_info->p_last_algo_data;
    }

    if (p_frame_info->pack_header.agc_info_en)
    {
        for (uint8_t i = 0; i < p_frame_info->agc_info_size; i++)
        {
            gh_agc_union_t agc_info_u = agc_info_combine(p_func_frame, i);
            p_bytes_info->p_last_agc_info[i] = agc_info_u.data32bit[0];
            p_bytes_info->p_last_agc_info_high[i] = agc_info_u.data32bit[1];

            //DEBUG_LOG("g_last_agc_info[%d]: %d \n", i, p_bytes_info->p_last_agc_info[i]);
        }
        p_frame_info->p_agc_info = p_bytes_info->p_last_agc_info;
        p_frame_info->p_agc_info_high = p_bytes_info->p_last_agc_info_high;
    }

    if (p_frame_info->pack_header.gs_data_en)
    {
        for (uint8_t i = 0; i < p_frame_info->gs_data_size; i++)
        {
            if (i < GH_ACC_AXIS_NUM)
            {
                p_bytes_info->p_last_gs_data[i] = p_func_frame->gsensor_data.acc[i];
            }
            else if (i >= GH_ACC_AXIS_NUM)
            {
#if GH_GYRO_EN
                p_bytes_info->p_last_gs_data[i] = p_func_frame->gsensor_data.gyro[i - GH_ACC_AXIS_NUM];
#endif
            }

            //DEBUG_LOG("g_last_gs_data[%d]: %d \n", i, p_bytes_info->p_last_gs_data[i]);
        }
        p_frame_info->p_gs_data = p_bytes_info->p_last_gs_data;
    }

    if (p_frame_info->pack_header.timestamp_en)
    {
        gh_ts_union_t ts;
        ts.timestaple = p_func_frame->timestamp;

        p_bytes_info->p_last_timestamp[0] = ts.lld_comb.data;
        p_frame_info->timestamp = p_bytes_info->p_last_timestamp[0];
        p_bytes_info->p_last_timestamp_high[0] = ts.lld_comb.data_high;
        p_frame_info->timestamp_high = p_bytes_info->p_last_timestamp_high[0];

        //DEBUG_LOG("g_last_timestamp[%d]: %d \n", 0, p_bytes_info->p_last_timestamp[0]);
    }
}

static void gh_protocol_data_diff(gh_func_frame_t* p_func_frame, data_frame_t* p_frame_info,
                                  data_frame_bytes_info_t* p_bytes_info)
{
    if (p_frame_info->pack_header.rawdata_en)
    {
        for (uint8_t i = 0; i < p_frame_info->rawdata_size; i++)
        {
            p_bytes_info->p_rawdata_diff[i] = p_func_frame->p_data[i].rawdata - p_bytes_info->p_last_rawdata[i];

//            DEBUG_LOG("fun_id:%d, g_rawdata_diff[%d]:%d,data:%d,lastdata:%d \n", p_func_frame->id, i,
//            p_bytes_info->p_rawdata_diff[i], p_func_frame->p_data[i].rawdata, p_bytes_info->p_last_rawdata[i]);

            p_bytes_info->p_last_rawdata[i] = p_func_frame->p_data[i].rawdata;
        }
        p_frame_info->p_rawdata = p_bytes_info->p_rawdata_diff;
    }

    if (p_frame_info->pack_header.phy_value_en)
    {
        for (uint8_t i = 0; i < p_frame_info->phy_value_size; i++)
        {
            p_bytes_info->p_phy_value_diff[i] = p_func_frame->p_data[i].ipd_pa - p_bytes_info->p_last_phy_value[i];

//          DEBUG_LOG("g_phy_value_diff[%d]: %d, data: %d, lastdata; %d \n", i,
//          p_bytes_info->p_phy_value_diff[i], p_func_frame->p_data[i].ipd_pa, p_bytes_info->p_last_phy_value[i]);

            p_bytes_info->p_last_phy_value[i] = p_func_frame->p_data[i].ipd_pa;
        }
        p_frame_info->p_phy_value = p_bytes_info->p_phy_value_diff;
    }

    if (p_frame_info->pack_header.gs_data_en)
    {
        for (uint8_t i = 0; i < p_frame_info->gs_data_size; i++)
        {
            if (i < GH_ACC_AXIS_NUM)
            {
                p_bytes_info->p_gs_data_diff[i] = p_func_frame->gsensor_data.acc[i] - p_bytes_info->p_last_gs_data[i];
//                DEBUG_LOG("g_gs_data_diff[%d]: %d,data: %d,lastdata; %d \n", i, p_bytes_info->p_gs_data_diff[i],
//                p_func_frame->gsensor_data.acc[i],p_bytes_info->p_last_gs_data[i]);
                p_bytes_info->p_last_gs_data[i] = p_func_frame->gsensor_data.acc[i];
            }
            else if (i >= GH_ACC_AXIS_NUM)
            {
#if GH_GYRO_EN
                p_bytes_info->p_gs_data_diff[i] = p_func_frame->gsensor_data.gyro[i - GH_ACC_AXIS_NUM] -
                    p_bytes_info->p_last_gs_data[i];

                //DEBUG_LOG("g_gs_data_diff[%d]: %d, data: %d, lastdata; %d \n", i,
                //p_bytes_info->p_gs_data_diff[i], p_func_frame->gsensor_data.gyro[i - GH_ACC_AXIS_NUM],
                //p_bytes_info->p_last_gs_data[i]);
                p_bytes_info->p_last_gs_data[i] = p_func_frame->gsensor_data.gyro[i - GH_ACC_AXIS_NUM];
#endif
            }

        }
        p_frame_info->p_gs_data = p_bytes_info->p_gs_data_diff;
    }

    if (p_frame_info->pack_header.timestamp_en)
    {
        gh_ts_union_t ts;
        ts.lld_comb.data = p_bytes_info->p_last_timestamp[0];
        ts.lld_comb.data_high = p_bytes_info->p_last_timestamp_high[0];
        uint64_t last_timestamp = ts.timestaple;
        p_bytes_info->p_timestamp_diff[0] = p_func_frame->timestamp - last_timestamp;

        //DEBUG_LOG("g_timestamp_diff[%d]: %d, data: %d, lastdata; %d \n", 0, p_bytes_info->p_timestamp_diff[0],
        //p_func_frame->timestamp, p_bytes_info->p_last_timestamp[0]);

        ts.timestaple = p_func_frame->timestamp;
        p_bytes_info->p_last_timestamp[0] = ts.lld_comb.data;
        p_bytes_info->p_last_timestamp_high[0] = ts.lld_comb.data_high;
        p_frame_info->timestamp = p_bytes_info->p_timestamp_diff[0];
    }

}

//uint8_t change_data_update(uint8_t count, int32_t* last_data, int32_t* current_data)
//{
//    uint8_t enable = 0;
//    for (uint8_t i = 0; i < count; i++)
//    {
//        if (last_data[i] != current_data[i])
//        {
//            last_data[i] = current_data[i];
//            enable = 1;
//        }
//    }
//
//    return enable;
//}


static uint8_t gh_protocol_algres_update(gh_func_frame_t* p_frame)
{
    uint8_t algres_num = 0;

    if (GH_NULL_PTR == p_frame->p_algo_res)
    {
        return algres_num;
    }

    switch (p_frame->id)
    {
        case GH_FUNC_FIX_IDX_ADT:
        {
            algres_num = ALG_RES_ADT_NUM;
            gh_algo_adt_result_t *p_adt_res = (gh_algo_adt_result_t *)p_frame->p_algo_res;
            g_algo_data[ADT_WEAR_EVENT_IDX] = p_adt_res->wear_evt;
            g_algo_data[ADT_WEAR_DET_STATE_IDX] = p_adt_res->det_status;
            g_algo_data[ADT_WEAR_CTR_IDX] = p_adt_res->ctr;
//            DEBUG_LOG("[ADT] wear_evt:%d, det_status:%d, ctr:%d\r\n", p_adt_res->wear_evt,
//                             p_adt_res->det_status, p_adt_res->ctr);
        }
        break;
#if (GH_USE_GOODIX_HR_ALGO)
        case GH_FUNC_FIX_IDX_HR:
        {
            algres_num = ALG_RES_HR_NUM;
            gh_algo_hr_result_t *p_hr_res = (gh_algo_hr_result_t *)p_frame->p_algo_res;
            gh_func_algo_param_t *p_hr_param = (gh_func_algo_param_t *)p_frame->p_algo_input;
            gh_algo_hr_t *p_hr = (gh_algo_hr_t *)p_hr_param->p_algo_inst;
            g_algo_data[HR_HBA_OUT_IDX] = p_hr_res->hba_out;
            g_algo_data[HR_VALID_SCORE_IDX] = p_hr_res->valid_score;
            g_algo_data[HR_SNR_IDX] = p_hr_res->hba_snr;
            g_algo_data[HR_ACC_INFO_IDX] = p_hr_res->hba_acc_info;
            g_algo_data[HR_REG_SCENCE_IDX] = p_hr_res->hba_reg_scence;
            g_algo_data[HR_INPUT_SCENCE_IDX] = p_hr->scence;
            g_algo_data[HR_RESERVED1] = p_hr_res->reserved1;
            g_algo_data[HR_RESERVED2] = p_hr_res->reserved2;
            g_algo_data[HR_RESERVED3] = p_hr_res->reserved3;

        }
        break;
#endif
#if (GH_USE_GOODIX_SPO2_ALGO)
        case GH_FUNC_FIX_IDX_SPO2:
        {
            algres_num = ALG_RES_SPO2_NUM;
            gh_algo_spo2_result_t *p_spo2_res = (gh_algo_spo2_result_t *)p_frame->p_algo_res;
            g_algo_data[SPO2_FINAL_SPO2_IDX] = p_spo2_res->final_spo2;
            g_algo_data[SPO2_R_VAL_IDX] = p_spo2_res->final_r_val;
            g_algo_data[SPO2_CONFI_COEFF_IDX] = p_spo2_res->final_confi_coeff;
            g_algo_data[SPO2_VALID_LEVEL_IDX] = p_spo2_res->final_valid_level;
            g_algo_data[SPO2_HB_MEAN_IDX] = p_spo2_res->final_hb_mean;
            g_algo_data[SPO2_INVALID_FLAG_IDX] = p_spo2_res->final_invalidFlg;
            g_algo_data[SPO2_RESERVED1] = p_spo2_res->reserved1;
            g_algo_data[SPO2_RESERVED2] = p_spo2_res->reserved2;
            g_algo_data[SPO2_RESERVED3] = p_spo2_res->reserved3;

        }
        break;
#endif
#if (GH_USE_GOODIX_HRV_ALGO)
        case GH_FUNC_FIX_IDX_HRV:
        {
            algres_num = ALG_RES_HRV_NUM;
            gh_algo_hrv_result_t *p_hrv_res = (gh_algo_hrv_result_t *)p_frame->p_algo_res;
            g_algo_data[HRV_RRI0_IDX] = p_hrv_res->rri[0];
            g_algo_data[HRV_RRI1_IDX] = p_hrv_res->rri[1];
            g_algo_data[HRV_RRI2_IDX] = p_hrv_res->rri[2];
            g_algo_data[HRV_RRI3_IDX] = p_hrv_res->rri[3];
            g_algo_data[HRV_CONFIDENCE_IDX] = p_hrv_res->rri_confidence;
            g_algo_data[HRV_VALID_NUM_IDX] = p_hrv_res->rri_valid_num;
            g_algo_data[HRV_RESERVED1] = p_hrv_res->reserved1;
            g_algo_data[HRV_RESERVED2] = p_hrv_res->reserved2;
            g_algo_data[HRV_RESERVED3] = p_hrv_res->reserved3;

        }
        break;
#endif
#if (GH_USE_GOODIX_NADT_ALGO)
        case GH_FUNC_FIX_IDX_GNADT:
        case GH_FUNC_FIX_IDX_IRNADT:
        {
            algres_num = ALG_RES_SOFT_ADT_NUM;
            gh_algo_nadt_result_t *p_nadt_res = (gh_algo_nadt_result_t *)p_frame->p_algo_res;
            g_algo_data[NADT_WEAR_OFF_RES_IDX] = p_nadt_res->nadt_out;
            g_algo_data[NADT_LIVE_BODY_CONF_IDX] = p_nadt_res->nadt_confi;
            g_algo_data[NADT_RESERVED1] = p_nadt_res->reserved1;
            g_algo_data[NADT_RESERVED1] = p_nadt_res->reserved2;
            g_algo_data[NADT_RESERVED1] = p_nadt_res->reserved3;

        }
        break;
#endif
        default:
        break;
    }//switch (p_frame->id)

    return algres_num;
}


static void gh_protocol_publish(data_frame_bytes_info_t* p_bytes_info)
{
    if (p_bytes_info->bytes_buffer_used  > 0)
    {
        // send
        RPCPoint rpcpoint = { p_bytes_info->p_bytes_buffer, p_bytes_info->bytes_buffer_used };
        GHRPC_publish("G", "<u8*>", rpcpoint);

        // send finish
        p_bytes_info->bytes_buffer_used = 0;
        memset(p_bytes_info->p_bytes_buffer, 0, p_bytes_info->bytes_buffer_len);

        p_bytes_info->start_flag = 1;
    }
}

uint32_t gh_protocol_reset(gh_func_fix_idx_e id)
{

    //DEBUG_LOG("[flush] id:%d, current_id:%d \n", id, g_fun_id);

    if (id != g_fun_id)
    {
        return 0;
    }

    data_frame_bytes_info_t* p_bytes_info = &g_bytes_info;
    gh_protocol_publish(p_bytes_info);

    return 1;
}

void gh_protocol_process(gh_func_frame_t* p_func_frame)
{
    data_frame_bytes_info_t* p_bytes_info = &g_bytes_info;
    data_frame_t frame_info = { 0 };

    if (GH_NULL_PTR == p_func_frame)
    {
        return;
    }

    if (g_fun_id != p_func_frame->id)
    {
        // DEBUG_LOG("funtion change, g_fun_id:%d, last_g_fun_id:%d \n", p_func_frame->id, g_fun_id);
        gh_protocol_publish(p_bytes_info);
        p_bytes_info->start_flag = 1;
    }
    g_fun_id = p_func_frame->id;

    if (1 == p_bytes_info->start_flag)
    {
        gh_memset(g_last_rawdata, 0, sizeof(g_last_rawdata));
        gh_memset(g_rawdata_diff, 0, sizeof(g_rawdata_diff));
        gh_memset(g_last_phy_value, 0, sizeof(g_last_phy_value));
        gh_memset(g_phy_value_diff, 0, sizeof(g_phy_value_diff));
        gh_memset(g_last_timestamp, 0, sizeof(g_last_timestamp));
        gh_memset(g_last_timestamp_high, 0, sizeof(g_last_timestamp_high));
        gh_memset(g_timestamp_diff, 0, sizeof(g_timestamp_diff));
        gh_memset(g_last_gs_data, 0, sizeof(g_last_gs_data));
        gh_memset(g_gs_data_diff, 0, sizeof(g_gs_data_diff));
        gh_memset(g_last_flags, 0, sizeof(g_last_flags));
        gh_memset(g_last_algo_data, 0, sizeof(g_last_algo_data));
        gh_memset(g_last_agc_info, 0, sizeof(g_last_agc_info));
        gh_memset(g_last_agc_info_high, 0, sizeof(g_last_agc_info_high));
        gh_memset(g_algo_data, 0, sizeof(g_algo_data));

        g_bytes_info.p_rawdata_diff = g_rawdata_diff;
        g_bytes_info.p_last_rawdata = g_last_rawdata;
        g_bytes_info.p_last_phy_value = g_last_phy_value;
        g_bytes_info.p_phy_value_diff = g_phy_value_diff;
        g_bytes_info.p_last_timestamp = g_last_timestamp;
        g_bytes_info.p_last_timestamp_high = g_last_timestamp_high;
        g_bytes_info.p_timestamp_diff = g_timestamp_diff;
        g_bytes_info.p_last_gs_data = g_last_gs_data;
        g_bytes_info.p_gs_data_diff = g_gs_data_diff;

        g_bytes_info.p_last_flags = g_last_flags;
        g_bytes_info.p_last_algo_data = g_last_algo_data;
        g_bytes_info.p_last_agc_info = g_last_agc_info;
        g_bytes_info.p_last_agc_info_high = g_last_agc_info_high;

        g_bytes_info.p_bytes_buffer = g_bytes_buffer;
        g_bytes_info.bytes_buffer_len = sizeof(g_bytes_buffer);
        g_bytes_info.bytes_buffer_used = 0;
    }//if (1 == p_bytes_info->start_flag)

    frame_info.rawdata_size = (GH_NULL_PTR != p_func_frame->p_data) ? p_func_frame->ch_num : 0;
    frame_info.phy_value_size = (GH_NULL_PTR != p_func_frame->p_data) ? p_func_frame->ch_num : 0;
    frame_info.gs_data_size = GH_GSENSOR_MAX;
    frame_info.flag_data_bits = (GH_NULL_PTR != p_func_frame->p_data) ? p_func_frame->ch_num : 0;
    frame_info.agc_info_size = (GH_NULL_PTR != p_func_frame->p_data) ? p_func_frame->ch_num : 0;

    frame_info.algo_data_bits = gh_protocol_algres_update(p_func_frame);

    frame_info.pack_header.rawdata_en = (frame_info.rawdata_size != 0) ? 1 : 0;
    frame_info.pack_header.phy_value_en = (frame_info.phy_value_size != 0) ? 1 : 0;
    frame_info.pack_header.gs_data_en = (p_func_frame->gsensor_en != 0) ? 1 : 0;

    frame_info.pack_header.timestamp_en = 1;
    frame_info.pack_header.func_id_en = 1;
    frame_info.pack_header.reserved = 0;

    frame_info.function_id = p_func_frame->id;
    frame_info.frame_id = p_func_frame->frame_cnt;

    //  update slotcfg_num
    if (GH_FUNC_FIX_IDX_PPG_CFG0 <= p_func_frame->id && GH_FUNC_FIX_IDX_CAP_CFG >= p_func_frame->id)
    {
        frame_info.pack_header.slot_cfg_en = 1;
        if (GH_FUNC_FIX_IDX_CAP_CFG == p_func_frame->id)
        {
            frame_info.slot_cfg = SLOT_CFG_CAP;
            frame_info.phy_value_size = 0;
            frame_info.agc_info_size  = 0;
        }
        else
        {
            frame_info.slot_cfg = p_func_frame->id - SLOT_CFG_BIAS;
        }

        // DEBUG_LOG("slot fucntion, g_fun_id:%d, slotcfg:%d \n", p_func_frame->id, frame_info.slot_cfg);
        frame_info.function_id = GH_FUNC_FIX_IDX_EVK;
    }

#if GH_BYTES_STREAM_DIFF_EN
    if (!p_bytes_info->start_flag)
    {
        gh_protocol_data_diff((gh_func_frame_t*)p_func_frame, &frame_info, p_bytes_info);

        ////upload data when change:  flags, algo_data, agc_info
        for (uint8_t i = 0; i < frame_info.flag_data_bits; i++)
        {
            g_flag_u.flag = p_func_frame->p_data[i].flag;
            if (p_bytes_info->p_last_flags[i] != g_flag_u.value)
            {
                p_bytes_info->p_last_flags[i] = g_flag_u.value;
                frame_info.pack_header.flags_en = 1;
            }
        }
        frame_info.p_flags = p_bytes_info->p_last_flags;

        for (uint8_t i = 0; i < frame_info.algo_data_bits; i++)
        {
//            DEBUG_LOG("fun:%d, res-%d: %d, last_res:%d \r\n", p_func_frame->id, i,
//                g_algo_data[i], p_bytes_info->p_last_algo_data[i]);
            if (p_bytes_info->p_last_algo_data[i] != g_algo_data[i])
            {
                p_bytes_info->p_last_algo_data[i] = g_algo_data[i];
                frame_info.pack_header.alg_data_en = 1;
            }
        }
        frame_info.p_algo_data = p_bytes_info->p_last_algo_data;

        for (uint8_t i = 0; i < frame_info.agc_info_size; i++)
        {
            gh_agc_union_t agc_info_u = agc_info_combine(p_func_frame, i);
            if (p_bytes_info->p_last_agc_info[i] != agc_info_u.data32bit[0]
                || p_bytes_info->p_last_agc_info_high[i] != agc_info_u.data32bit[1])
            {
                p_bytes_info->p_last_agc_info[i] = agc_info_u.data32bit[0];
                p_bytes_info->p_last_agc_info_high[i] = agc_info_u.data32bit[1];
                frame_info.pack_header.agc_info_en = 1;
            }
        }
        frame_info.p_agc_info = p_bytes_info->p_last_agc_info;
        frame_info.p_agc_info_high = p_bytes_info->p_last_agc_info_high;
    }//if (!p_bytes_info->start_flag)
    else
    {
        p_bytes_info->start_flag = 0;
        frame_info.pack_header.flags_en = (frame_info.flag_data_bits != 0) ? 1 : 0;
        frame_info.pack_header.alg_data_en = (frame_info.algo_data_bits != 0) ? 1 : 0;
        frame_info.pack_header.agc_info_en = (frame_info.agc_info_size != 0) ? 1 : 0;
        gh_protocol_lastdata_update((gh_func_frame_t*)p_func_frame, &frame_info, p_bytes_info);
    }
#else

#endif

    p_bytes_info->bytes_buffer_used +=
        gh_protocol_rawdata_to_bytes(&frame_info, &p_bytes_info->p_bytes_buffer[p_bytes_info->bytes_buffer_used],
                                     p_bytes_info->bytes_buffer_len);

    // send bytes stream when reach 200 bytes or fifo end
    if ((p_bytes_info->bytes_buffer_used >= BUFFER_BYTE_THRD) || (p_func_frame->fifo_end_flag))
    {
//        DEBUG_LOG("buffer_used:%d, fifo_end:%d \n",
//            p_bytes_info->bytes_buffer_used, p_func_frame->fifo_end_flag);
        gh_protocol_publish(p_bytes_info);
    }
}

/*   decode function of receiver
*/
#define     GET_BIT06_MASK     (0x7f)

static int32_t zigzag_decode(uint32_t x)
{
    return (int32_t)((x >> 1) ^ ((~(x & 1)) + 1));
}

void gh_protocol_bytes_read(uint8_t* buffer, int32_t* pos, int32_t* dataOut, int32_t data_cnt)
{
    uint32_t num = 0;
    uint8_t cur_byte = 0;
    int32_t shift = 0;

    for (int32_t cnt = 0; cnt < data_cnt; cnt++)
    {
        do
        {
            cur_byte = buffer[*pos];
            num |= (cur_byte & GET_BIT06_MASK) << shift;
            shift += SHIFT_BIT_NUM;
            (*pos)++;
        } while (cur_byte & 0x80);


        dataOut[cnt] = zigzag_decode(num);
        num = 0;
        shift = 0;
    }

    return;
}

int32_t gh_protocol_bytes_to_rawdata(data_frame_t* data, uint8_t* buffer, int32_t buffer_size)
{
    int32_t pos = 0;

    gh_protocol_bytes_read(buffer, &pos, (int32_t*)&data->pack_header,
         sizeof(data->pack_header) / sizeof(pack_header_t));
    if (data->pack_header.rawdata_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->rawdata_size, sizeof(data->rawdata_size) / sizeof(int32_t));
        gh_protocol_bytes_read(buffer, &pos, data->p_rawdata, data->rawdata_size);
    }

    if (data->pack_header.gs_data_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->gs_data_size, sizeof(data->gs_data_size) / sizeof(int32_t));
        gh_protocol_bytes_read(buffer, &pos, data->p_gs_data, data->gs_data_size);
    }

    if (data->pack_header.flags_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->flag_data_bits, sizeof(data->flag_data_bits) / sizeof(int32_t));
        gh_protocol_bytes_read(buffer, &pos, data->p_flags, data->flag_data_bits);
    }

    if (data->pack_header.alg_data_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->algo_data_bits, sizeof(data->algo_data_bits) / sizeof(int32_t));
        gh_protocol_bytes_read(buffer, &pos, data->p_algo_data, data->algo_data_bits);
    }

    if (data->pack_header.agc_info_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->agc_info_size, sizeof(data->agc_info_size) / sizeof(int32_t));
        gh_protocol_bytes_read(buffer, &pos, data->p_agc_info, data->agc_info_size);
    }

    gh_protocol_bytes_read(buffer, &pos, &data->frame_id, sizeof(data->frame_id) / sizeof(int32_t));

    if (data->pack_header.func_id_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->function_id, sizeof(data->function_id) / sizeof(int32_t));
    }

    if (data->pack_header.slot_cfg_en)
    {
        gh_protocol_bytes_read(buffer, &pos, &data->slot_cfg, sizeof(data->slot_cfg) / sizeof(int32_t));
    }

    if (pos > buffer_size)
    {
        // warning
        return buffer_size;
    }

    return pos;
}


