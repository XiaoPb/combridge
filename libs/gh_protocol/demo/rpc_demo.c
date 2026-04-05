#include "gh_rpccore.h"
#include "rpc_demo.h"

rpc_api_t rpc_api;


void RPC_GetVersion(uint8_t uchVerType,uint8_t* pchVer,uint16_t* size)
{
    uint8_t ret[256];
    RPCPoint pver = {pchVer, 0};
    GHRPC_call(ret, "GH3X_GetVersion","<u8>",uchVerType);
    GHRPC_unpack(ret,"<u8*>",&pver);
    *size = (uint16_t)pver.size;
}

void RPC_RegsWriteCmd(uint16_t* pusRegs,int32_t nSize)
{
    RPCPoint pver = {pusRegs, (size_t)nSize};
    GHRPC_send("GH3X_RegsWriteCmd","<u16*>",&pver);
}

void RPC_RegsReadCmd(uint16_t usRegAddr,int32_t nReadLen,uint16_t* pusRegValueBuffer,int32_t* pnLen)
{
    uint8_t ret[256];
    RPCPoint pregs = {pusRegValueBuffer, 0};
    GHRPC_call(ret, "GH3X_RegsReadCmd","<u16><d32>",usRegAddr,nReadLen);
    GHRPC_unpack(ret,"<u16*>",&pregs);
    *pnLen = (int32_t)pregs.size;
}

void RPC_RegBitFieldWriteCmd(uint16_t usRegAddr,uint8_t uchLsb,uint8_t uchMsb,uint16_t usRegVal)
{
    GHRPC_send("GH3X_RegBitFieldWriteCmd","<u16><u8><u8><u16>",usRegAddr,uchLsb,uchMsb,usRegVal);
}

void RPC_ChipCtrl(uint8_t uchCtrlType)
{
    GHRPC_send("GH3X_ChipCtrl","<u8>",uchCtrlType);
}

void download_config(uint8_t uchStage)
{
    GHRPC_send("GH3X_DownloadConfig","<u8>",uchStage);
}

void RPC_RegsListWriteCmd(uint16_t* usRegs,uint16_t usLen)
{
    RPCPoint pregs = {usRegs, (size_t)usLen};
    GHRPC_send("GH3X_RegsListWriteCmd","<u16*>",&pregs);
}


void RPC_SwFunctionCmd(uint32_t unTargetFuncMode,uint8_t uchCtrlType)
{
    GHRPC_send("GH3X_SwFunctionCmd","<u32><u8>",unTargetFuncMode,uchCtrlType);
}
void gh_low_power_cmd(uint32_t unTargetFuncMode,uint8_t uchCtrlType)
{
    GHRPC_send("GH3X_LowPowerCmd","<u32><u8>",unTargetFuncMode,uchCtrlType);
}

void RPC_FwUpdateCmd(uint8_t* pSrc,uint32_t usLen,uint8_t* puchRet,uint32_t* pRetLen)
{
    uint8_t ret[256];
    RPCPoint pdata = {pSrc, (size_t)usLen};
    RPCPoint pret = {puchRet, 0};
    GHRPC_call(ret, "GH3X_FwUpdateCmd","<u8*>",&pdata);
    GHRPC_unpack(ret,"<u8*>",&pret);
    *pRetLen = (uint32_t)pret.size;
}

void RPC_RegsBitFieldWriteCmd(uint16_t* usRegBits,uint16_t size)
{
    RPCPoint pver = {usRegBits, (size_t)size};
    GHRPC_send("GH3X_RegsBitFieldWriteCmd","<u16*>",&pver);
}

void GHSetWorkModeCmd(uint8_t uchWorkMode)
{
    GHRPC_send("GH3X_SetWorkModeCmd","<u8>",uchWorkMode);
}
void get_chip_link_status(uint8_t type,int8_t* pusStatus,int32_t* pnLen)
{    
    uint8_t ret[256];
    RPCPoint pstatus = {(uint8_t*)pusStatus, 0};
    GHRPC_call(ret, "GH3X_GetChipLinkStatus","<u8>",type);
    GHRPC_unpack(ret,"<u8*>",&pstatus);
    *pnLen = (int32_t)pstatus.size;
}

void gh_timestamp_set(uint32_t ts)
{
    GHRPC_send("GH3X_TimestampSet","<u32>",ts);
}

void gh_time_set(uint32_t ts,int8_t hour_offset)
{
    GHRPC_send("GH3X_TimeSet","<u32><u8>",ts,hour_offset);  
}

void RPC_Receive(uint8_t* pData,uint32_t len)
{
    GHRPC_process(pData,(int)len, 0);
}

void DealFrameDataProcess(uint8_t* buf,int size)
{
    data_frame_t data;

    int ret = gh_protocol_bytes_to_rawdata(&data, buf, size);
    if (ret < 0)
    {
        return;
    }
}

rpc_api_t* rpc_init(GhRPCInitialInfo* info, Func_Handle event_handle)
{
    rpc_api.info.lock = info->lock;
    rpc_api.info.unlock = info->unlock;
    rpc_api.info.sendFunction = info->sendFunction;
    rpc_api.info.delay = info->delay;
    rpc_api.info.mode = info->mode;

    GHRPC_init(rpc_api.info);

    rpc_api.RPC_GetVersion = RPC_GetVersion;
    rpc_api.RPC_RegsWriteCmd = RPC_RegsWriteCmd;
    rpc_api.RPC_RegsReadCmd = RPC_RegsReadCmd;
    rpc_api.RPC_RegBitFieldWriteCmd = RPC_RegBitFieldWriteCmd;
    rpc_api.RPC_ChipCtrl = RPC_ChipCtrl;
    rpc_api.RPC_DownloadConfig = download_config;
    rpc_api.RPC_RegsListWriteCmd = RPC_RegsListWriteCmd;
    rpc_api.RPC_SwFunctionCmd = RPC_SwFunctionCmd;
    rpc_api.RPC_LowPowerCmd = gh_low_power_cmd;
    rpc_api.RPC_FwUpdateCmd = RPC_FwUpdateCmd;
    rpc_api.RPC_RegsBitFieldWriteCmd = RPC_RegsBitFieldWriteCmd;
    rpc_api.RPC_SetWorkModeCmd = GHSetWorkModeCmd;
    rpc_api.RPC_GetChipLinkStatus = get_chip_link_status;
    rpc_api.RPC_GhTimestampSet = gh_timestamp_set;
    rpc_api.RPC_GhTimeSet = gh_time_set;
    rpc_api.RPC_EventHandle = event_handle;
    rpc_api.RPC_Receive = RPC_Receive;


    return &rpc_api;
}
