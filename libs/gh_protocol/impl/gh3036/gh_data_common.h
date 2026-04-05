/**
  ****************************************************************************************
* @file    gh_data_common.h
* @author  GHealth Driver Team
* @brief   ghealth data common definitions
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

#ifndef __GH_DATA_COMMON_H__
#define __GH_DATA_COMMON_H__

#include <stdint.h>
#include "gh_config.h"

#ifdef __cplusplus
extern "C"
{
#endif

#define GH_ACC_AXIS_NUM                    (3)
#define GH_GYRO_AXIS_NUM                   (3)
#define GH_LED_DRV_NUM                     (2)
#define GH_FUNC_NAME_LEN                   (25)

#pragma pack(push, 1)
typedef struct
{
    uint32_t gain_code : 4;
    uint32_t bg_cancel_range : 2;
    uint32_t dc_cancel_range : 2;
    uint32_t dc_cancel_code : 8;
    uint32_t led_drv0 : 8;
    uint32_t led_drv1 : 8;
    uint32_t bg_cancel_code : 8;
    uint32_t tia_gain : 3;
    uint32_t resveresd : 5;
} gh_agc_info_t;

typedef struct
{
    uint8_t led_adj_flag : 1;
    uint8_t sa_flag : 1;
    uint8_t param_change_flag : 1;
    uint8_t dre_update : 1;
    uint8_t skip_ok_flag : 1;
    uint8_t resveresd : 3;
} gh_frame_data_flag_t;

typedef struct
{
    int32_t ipd_pa;
    int32_t rawdata;
    gh_frame_data_flag_t flag;
    gh_agc_info_t agc_info;
} gh_frame_data_t;
#pragma pack(pop)

typedef enum
{
    GH_FUNC_FIX_IDX_ADT = 0,
    GH_FUNC_FIX_IDX_HR = 1,
    GH_FUNC_FIX_IDX_SPO2 = 2,
    GH_FUNC_FIX_IDX_HRV = 3,
    GH_FUNC_FIX_IDX_GNADT = 4,
    GH_FUNC_FIX_IDX_IRNADT = 5,
    GH_FUNC_FIX_IDX_ALGO_MAX = 6,

    GH_FUNC_FIX_IDX_TEST1 = 6,
    GH_FUNC_FIX_IDX_TEST2 = 7,
    GH_FUNC_FIX_IDX_PPG_CFG0 = 8,
    GH_FUNC_FIX_IDX_PPG_CFG1 = 9,
    GH_FUNC_FIX_IDX_PPG_CFG2 = 10,
    GH_FUNC_FIX_IDX_PPG_CFG3 = 11,
    GH_FUNC_FIX_IDX_PPG_CFG4 = 12,
    GH_FUNC_FIX_IDX_PPG_CFG5 = 13,
    GH_FUNC_FIX_IDX_PPG_CFG6 = 14,
    GH_FUNC_FIX_IDX_PPG_CFG7 = 15,
    GH_FUNC_FIX_IDX_CAP_CFG = 16,
    GH_FUNC_FIX_IDX_MAX
} gh_func_fix_idx_e;

typedef enum
{
    GH_FUNC_FIX_ADT = 1 << GH_FUNC_FIX_IDX_ADT,
    GH_FUNC_FIX_HR = 1 << GH_FUNC_FIX_IDX_HR,
    GH_FUNC_FIX_SPO2 = 1 << GH_FUNC_FIX_IDX_SPO2,
    GH_FUNC_FIX_HRV = 1 << GH_FUNC_FIX_IDX_HRV,
    GH_FUNC_FIX_GNADT = 1 << GH_FUNC_FIX_IDX_GNADT,
    GH_FUNC_FIX_IRNADT = 1 << GH_FUNC_FIX_IDX_IRNADT,
    GH_FUNC_FIX_TEST1 = 1 << GH_FUNC_FIX_IDX_TEST1,
    GH_FUNC_FIX_TEST2 = 1 << GH_FUNC_FIX_IDX_TEST2,
    GH_FUNC_FIX_PPG_CFG0 = 1 << GH_FUNC_FIX_IDX_PPG_CFG0,
    GH_FUNC_FIX_PPG_CFG1 = 1 << GH_FUNC_FIX_IDX_PPG_CFG1,
    GH_FUNC_FIX_PPG_CFG2 = 1 << GH_FUNC_FIX_IDX_PPG_CFG2,
    GH_FUNC_FIX_PPG_CFG3 = 1 << GH_FUNC_FIX_IDX_PPG_CFG3,
    GH_FUNC_FIX_PPG_CFG4 = 1 << GH_FUNC_FIX_IDX_PPG_CFG4,
    GH_FUNC_FIX_PPG_CFG5 = 1 << GH_FUNC_FIX_IDX_PPG_CFG5,
    GH_FUNC_FIX_PPG_CFG6 = 1 << GH_FUNC_FIX_IDX_PPG_CFG6,
    GH_FUNC_FIX_PPG_CFG7 = 1 << GH_FUNC_FIX_IDX_PPG_CFG7,
    GH_FUNC_FIX_CAP_CFG = 1 << GH_FUNC_FIX_IDX_CAP_CFG,
} gh_func_fix_id_e;

#pragma pack(push, 1)
typedef struct
{
    int16_t acc[GH_ACC_AXIS_NUM];
#if GH_GYRO_EN
    int16_t gyro[GH_GYRO_AXIS_NUM];
#endif
} gh_gsensor_data_t;
#pragma pack(pop)

typedef struct
{
    uint64_t timestamp;
    gh_gsensor_data_t data;
} gh_gsensor_ts_and_data_t;

typedef struct gh_hal_data_channel_t gh_hal_data_channel_t;

typedef struct
{
    uint32_t frame_cnt;
    uint64_t timestamp;
#if GH_GSENSOR_DEBUG_EN
    uint64_t gs_left_ts;
    uint64_t gs_right_ts;
#endif
    gh_gsensor_data_t gsensor_data;
    gh_func_fix_idx_e id;
    gh_hal_data_channel_t *p_ch_map;
    gh_frame_data_t *p_data;
    uint8_t ch_num;
    uint8_t ch_max;
    uint8_t gsensor_en;
    uint8_t fifo_end_flag;
    uint8_t led_drv_fs[GH_LED_DRV_NUM];
    void *p_algo_res;
    void *p_algo_input;
} gh_func_frame_t;

typedef uint32_t (*gh_frame_publish_t)(void *p_parent_node, gh_func_frame_t *frame);

#ifdef __cplusplus
}
#endif

#endif
