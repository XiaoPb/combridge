/**
 ****************************************************************************************
 *
 * @file    gh_protocol_user.h
 * @author  GOODIX GH Driver Team
 * @brief   Header file containing gh protocol user interface.
 *
 ****************************************************************************************
 * @attention
  #####Copyright (c) 2019 GOODIX
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

/** @addtogroup GH protocol user
  * @{
  */

/** @defgroup API
  * @brief Protocol API.
  * @{
  */

/* Define to prevent recursive inclusion -------------------------------------*/

#ifndef __GH_PROTOCOL_USER_H__
#define __GH_PROTOCOL_USER_H__

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

/**
 * @brief Goodix protocol initialization
 *
 * @return None
 *
 * @note None
 */
extern void gh_protocol_init(void);

/**
 * @brief Goodix protocol processing received data
 *
 * @param p_rx_buffer: pointer of data buffer
 * @param rx_len: data buffer length
 * @return None
 *
 * @note When receiving data from the GOODIX PC host tool, call this function to process the data
 */
extern void gh_protocol_data_recevice(uint8_t *p_rx_buffer, uint8_t rx_len);

#ifdef __cplusplus
}
#endif

#endif /* __GH_PROTOCOL_USER_H__ */

/** @} */

/** @} */

