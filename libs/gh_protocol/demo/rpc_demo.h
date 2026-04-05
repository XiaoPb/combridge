#ifndef RPC_DEMO_H
#define RPC_DEMO_H

#include "gh_rpccore.h"
#include "gh_data_package.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef void (*Func_U8_PU8_PU16)  (uint8_t, uint8_t*, uint16_t*);
typedef void (*Func_PU16_I32)    (uint16_t*, int32_t);
typedef void (*Func_U16_I32_PU16_PI32)(uint16_t, int32_t, uint16_t*, int32_t*);
typedef void (*Func_U16_U8_U8_U16)(uint16_t, uint8_t, uint8_t, uint16_t);
typedef void (*Func_U8)           (uint8_t);
typedef void (*Func_PU16_U16)    (uint16_t*, uint16_t);
typedef void (*Func_U32_U8)      (uint32_t, uint8_t);
typedef void (*Func_PU8_U32_PU8_PU32)(uint8_t*, uint32_t, uint8_t*, uint32_t*);
typedef void (*Func_U8_PI8_PI32) (uint8_t, int8_t*, int32_t*);
typedef void (*Func_U32)         (uint32_t);
typedef void (*Func_U32_I8)      (uint32_t, int8_t);

typedef void (*Func_Handle)      (uint8_t, uint8_t*, uint32_t);
typedef void (*Func_Receive)     (uint8_t*, uint32_t);

typedef struct rpc_api_t
{
    GhRPCInitialInfo info;

    Func_U8_PU8_PU16          RPC_GetVersion;
    Func_PU16_I32             RPC_RegsWriteCmd;
    Func_U16_I32_PU16_PI32    RPC_RegsReadCmd;
    Func_U16_U8_U8_U16        RPC_RegBitFieldWriteCmd;
    Func_U8                   RPC_ChipCtrl;
    Func_U8                   RPC_DownloadConfig;
    Func_PU16_U16             RPC_RegsListWriteCmd;
    Func_U32_U8               RPC_SwFunctionCmd;
    Func_U32_U8               RPC_LowPowerCmd;
    Func_PU8_U32_PU8_PU32     RPC_FwUpdateCmd;
    Func_PU16_U16             RPC_RegsBitFieldWriteCmd;
    Func_U8                   RPC_SetWorkModeCmd;
    Func_U8_PI8_PI32          RPC_GetChipLinkStatus;
    Func_U32                  RPC_GhTimestampSet;
    Func_U32_I8               RPC_GhTimeSet;

    Func_Handle               RPC_EventHandle;
    Func_Receive              RPC_Receive;

} rpc_api_t;


rpc_api_t* rpc_init(GhRPCInitialInfo* info, Func_Handle event_handle);

#ifdef __cplusplus
}
#endif

#endif
