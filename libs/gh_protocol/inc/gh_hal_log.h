#ifndef GH_HAL_LOG_H
#define GH_HAL_LOG_H

#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

#if GH_MODULE_PROTOCOL_LOG_EN
#define GH_LOG_LVL_DEBUG(fmt, ...) printf("[DEBUG] " fmt "\n", ##__VA_ARGS__)
#define GH_LOG_LVL_WARNING(fmt, ...) printf("[WARN] " fmt "\n", ##__VA_ARGS__)
#define GH_LOG_LVL_ERROR(fmt, ...) printf("[ERROR] " fmt "\n", ##__VA_ARGS__)
#else
#define GH_LOG_LVL_DEBUG(fmt, ...)
#define GH_LOG_LVL_WARNING(fmt, ...)
#define GH_LOG_LVL_ERROR(fmt, ...)
#endif

#ifdef __cplusplus
}
#endif

#endif
