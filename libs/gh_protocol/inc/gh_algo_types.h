#ifndef GH_ALGO_TYPES_H
#define GH_ALGO_TYPES_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int32_t wear_evt;
    int32_t det_status;
    int32_t ctr;
} gh_algo_adt_result_t;

#if GH_USE_GOODIX_HR_ALGO
typedef struct {
    int32_t hba_out;
    int32_t valid_score;
    int32_t hba_snr;
    int32_t hba_acc_info;
    int32_t hba_reg_scence;
    int32_t reserved1;
    int32_t reserved2;
    int32_t reserved3;
} gh_algo_hr_result_t;

typedef struct {
    void* p_algo_inst;
} gh_func_algo_param_t;

typedef struct {
    int32_t scence;
} gh_algo_hr_t;
#endif

#if GH_USE_GOODIX_SPO2_ALGO
typedef struct {
    int32_t final_spo2;
    int32_t final_r_val;
    int32_t final_confi_coeff;
    int32_t final_valid_level;
    int32_t final_hb_mean;
    int32_t final_invalidFlg;
    int32_t reserved1;
    int32_t reserved2;
    int32_t reserved3;
} gh_algo_spo2_result_t;
#endif

#if GH_USE_GOODIX_HRV_ALGO
typedef struct {
    int32_t rri[4];
    int32_t rri_confidence;
    int32_t rri_valid_num;
    int32_t reserved1;
    int32_t reserved2;
    int32_t reserved3;
} gh_algo_hrv_result_t;
#endif

#if GH_USE_GOODIX_NADT_ALGO
typedef struct {
    int32_t nadt_out;
    int32_t nadt_confi;
    int32_t reserved1;
    int32_t reserved2;
    int32_t reserved3;
} gh_algo_nadt_result_t;
#endif

#ifdef __cplusplus
}
#endif

#endif
