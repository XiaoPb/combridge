/**
  ****************************************************************************************
* @file    gh_data_package.h
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

/** @addtogroup GH API
  * @{
  */

/** @defgroup API
  * @brief Public API.
  * @{
  */

#ifndef _GH_DATA_PACKAGE_H_
#define _GH_DATA_PACKAGE_H_

#include <stdint.h>
#include "gh_data_common.h"

#ifdef __cplusplus
extern "C"
{
#endif

#pragma pack(push, 1)
typedef struct
{
    uint32_t rawdata_en : 1;
    uint32_t phy_value_en : 1;
    uint32_t gs_data_en : 1;
    uint32_t flags_en : 1;
    uint32_t alg_data_en : 1;
    uint32_t agc_info_en : 1;
    uint32_t timestamp_en : 1;
    uint32_t frameid_en : 1;
    uint32_t func_id_en : 1;
    uint32_t slot_cfg_en : 1;
    uint32_t reserved : 22;
} pack_header_t;

typedef struct
{
    pack_header_t pack_header;
    int32_t slot_cfg;
    int32_t function_id;
    int32_t frame_id;
    int32_t timestamp;
    int32_t timestamp_high;
    int32_t *p_agc_info;
    int32_t *p_agc_info_high;
    int32_t agc_info_size;
    int32_t *p_algo_data;
    int32_t algo_data_bits;
    int32_t *p_flags;
    int32_t flag_data_bits;
    int32_t *p_gs_data;
    int32_t gs_data_size;
    int32_t *p_phy_value;
    int32_t phy_value_size;
    int32_t *p_rawdata;
    int32_t rawdata_size;
} data_frame_t;

typedef struct
{
    int32_t *p_rawdata_diff;
    int32_t *p_last_rawdata;
    int32_t *p_phy_value_diff;
    int32_t *p_last_phy_value;
    int32_t *p_timestamp_diff;
    int32_t *p_last_timestamp;
    int32_t *p_last_timestamp_high;
    int32_t *p_gs_data_diff;
    int32_t *p_last_gs_data;
    int32_t *p_last_flags;
    int32_t *p_last_algo_data;
    int32_t *p_last_agc_info;
    int32_t *p_last_agc_info_high;
    uint8_t start_flag;
    uint8_t *p_bytes_buffer;
    uint32_t bytes_buffer_len;
    uint32_t bytes_buffer_used;
} data_frame_bytes_info_t;

typedef struct
{
    uint32_t gain_code : 4;
    uint32_t bg_cancel_range : 2;
    uint32_t dc_cancel_range : 2;
    uint32_t dc_cancel_code : 8;
    uint32_t led_drv_fs : 8;
    uint32_t led_drv0 : 8;
    uint32_t led_drv1 : 8;
    uint32_t led_drv2 : 8;
    uint32_t led_drv3 : 8;
} gh_agc_upload_t;

typedef union
{
    gh_agc_upload_t gh_agc_upload;
    int32_t data32bit[2];
} gh_agc_union_t;

typedef struct
{
    int32_t data;
    int32_t data_high;
} gh_lld_t;

typedef union
{
    uint64_t timestaple;
    gh_lld_t lld_comb;
} gh_ts_union_t;

typedef union
{
    gh_frame_data_flag_t flag;
    uint8_t value;
} gh_flag_union_t;
#pragma pack(pop)

typedef enum
{
    ADT_WEAR_EVENT_IDX = 0,
    ADT_WEAR_DET_STATE_IDX = 1,
    ADT_WEAR_CTR_IDX = 2,
    ALG_RES_ADT_NUM,
} gh_adt_alg_e;

typedef enum
{
    HR_HBA_OUT_IDX = 0,
    HR_VALID_SCORE_IDX = 1,
    HR_SNR_IDX = 2,
    HR_BLANK_IDX = 3,
    HR_ACC_INFO_IDX = 4,
    HR_REG_SCENCE_IDX = 5,
    HR_INPUT_SCENCE_IDX = 6,
    HR_RESERVED1 = 7,
    HR_RESERVED2 = 8,
    HR_RESERVED3 = 9,
    ALG_RES_HR_NUM,
} gh_hr_alg_e;

typedef enum
{
    SPO2_FINAL_SPO2_IDX = 0,
    SPO2_R_VAL_IDX = 1,
    SPO2_CONFI_COEFF_IDX = 2,
    SPO2_VALID_LEVEL_IDX = 3,
    SPO2_HB_MEAN_IDX = 4,
    SPO2_INVALID_FLAG_IDX = 5,
    SPO2_RESERVED1 = 6,
    SPO2_RESERVED2 = 7,
    SPO2_RESERVED3 = 8,
    ALG_RES_SPO2_NUM,
} gh_spo2_alg_e;

typedef enum
{
    HRV_RRI0_IDX = 0,
    HRV_RRI1_IDX = 1,
    HRV_RRI2_IDX = 2,
    HRV_RRI3_IDX = 3,
    HRV_CONFIDENCE_IDX = 4,
    HRV_VALID_NUM_IDX = 5,
    HRV_RESERVED1 = 6,
    HRV_RESERVED2 = 7,
    HRV_RESERVED3 = 8,
    ALG_RES_HRV_NUM,
} gh_spo2_hrv_e;

typedef enum
{
    NADT_WEAR_OFF_RES_IDX = 0,
    NADT_LIVE_BODY_CONF_IDX = 1,
    NADT_RESERVED1 = 2,
    NADT_RESERVED2 = 3,
    NADT_RESERVED3 = 4,
    ALG_RES_SOFT_ADT_NUM,
} gh_spo2_nadt_e;


void gh_protocol_process(gh_func_frame_t* p_func_frame);

uint32_t gh_protocol_reset(gh_func_fix_idx_e id);

int32_t gh_protocol_bytes_to_rawdata(data_frame_t* data, uint8_t* buffer, int32_t buffer_size);

#ifdef __cplusplus
}
#endif

#endif

/** @} */

/** @} */
