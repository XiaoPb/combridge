#ifndef GH_RPC_FUNCTIONS_H
#define GH_RPC_FUNCTIONS_H

#include "gh_rpccore.h"

#ifdef __cplusplus
extern "C" {
#endif

#pragma pack(push, 1)
typedef struct
{
    RPCPoint buf;
    uint32_t unFifoId;
} Struct_DealFifoDataProcess;
#pragma pack(pop)

extern void DealFifoDataProcess(uint8_t* buf, int size, uint32_t unFifoId);

static void WrapDealFifoDataProcess(Struct_DealFifoDataProcess* point)
{
    DealFifoDataProcess(point->buf.point, (int)point->buf.size, point->unFifoId);
}

static InvokeNode DealFifoDataProcess_node = {
    "F",
    "<u8*><u32>",
    NULL,
    (RpcFunction)WrapDealFifoDataProcess,
    NULL
};

#pragma pack(push, 1)
typedef struct
{
    RPCPoint buf;
} Struct_DealFrameDataProcess;
#pragma pack(pop)

extern void DealFrameDataProcess(uint8_t* buf, int size);

static void WrapDealFrameDataProcess(Struct_DealFrameDataProcess* point)
{
    DealFrameDataProcess(point->buf.point, (int)point->buf.size);
}

static InvokeNode DealFrameDataProcess_node = {
    "G",
    "<u8*>",
    NULL,
    (RpcFunction)WrapDealFrameDataProcess,
    NULL
};

#pragma pack(push, 1)
typedef struct
{
    RPCPoint buf;
} Struct_ChipEventProcess;
#pragma pack(pop)

extern void ChipEventProcess(uint8_t* buf, int size);

static void WrapChipEventProcess(Struct_ChipEventProcess* point)
{
    ChipEventProcess(point->buf.point, (int)point->buf.size);
}

static InvokeNode ChipEventProcess_node = {
    "Event",
    "<u8*>",
    NULL,
    (RpcFunction)WrapChipEventProcess,
    NULL
};

static InvokeNode* gh_static_nodes[] = {
    &ChipEventProcess_node, &DealFifoDataProcess_node, &DealFrameDataProcess_node
};

#ifdef __cplusplus
}
#endif

#endif
