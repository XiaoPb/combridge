#ifndef GH_CONFIG_H
#define GH_CONFIG_H

#ifdef __cplusplus
extern "C" {
#endif

#define GH_NULL_PTR ((void*)0)

#ifndef GH_GYRO_EN
#define GH_GYRO_EN 0
#endif

#ifndef GH_GSENSOR_DEBUG_EN
#define GH_GSENSOR_DEBUG_EN 0
#endif

#ifndef GH_MODULE_PROTOCOL_LOG_EN
#define GH_MODULE_PROTOCOL_LOG_EN 0
#endif

#ifndef GH_USE_GOODIX_HR_ALGO
#define GH_USE_GOODIX_HR_ALGO 1
#endif

#ifndef GH_USE_GOODIX_SPO2_ALGO
#define GH_USE_GOODIX_SPO2_ALGO 1
#endif

#ifndef GH_USE_GOODIX_HRV_ALGO
#define GH_USE_GOODIX_HRV_ALGO 01
#endif

#ifndef GH_USE_GOODIX_NADT_ALGO
#define GH_USE_GOODIX_NADT_ALGO 1
#endif

#ifndef GH_FRAME_RAWDATA_MAX_SIZE
#define GH_FRAME_RAWDATA_MAX_SIZE 32
#endif

#ifndef GH_FRAME_GS_DATA_MAX_SIZE
#define GH_FRAME_GS_DATA_MAX_SIZE 6
#endif

#ifndef GH_FRAME_FLAGS_MAX_SIZE
#define GH_FRAME_FLAGS_MAX_SIZE 32
#endif

#ifndef GH_FRAME_ALGO_DATA_MAX_SIZE
#define GH_FRAME_ALGO_DATA_MAX_SIZE 32
#endif

#ifndef GH_FRAME_AGC_INFO_MAX_SIZE
#define GH_FRAME_AGC_INFO_MAX_SIZE 32
#endif

#ifndef GH_FRAME_PHY_VALUE_MAX_SIZE
#define GH_FRAME_PHY_VALUE_MAX_SIZE 32
#endif

#ifndef GH_PROTOCOL_POLL_EN
#define GH_PROTOCOL_POLL_EN 0
#endif

#ifndef gh_memset
#include <string.h>
#define gh_memset memset
#endif

#ifndef gh_memcpy
#include <string.h>
#define gh_memcpy memcpy
#endif

#ifdef __cplusplus
}
#endif

#endif
