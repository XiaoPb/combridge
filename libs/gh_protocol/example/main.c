/**
 * @file main.c
 * @brief GH Protocol API 使用示例
 * 
 * 本示例展示如何在外部提供线程和串口收发处理的情况下使用 gh_protocol API。
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#endif

#include "gh_protocol_api.h"

#define RX_BUFFER_SIZE 4096
#define TX_BUFFER_SIZE 4096

static uint8_t g_rx_buffer[RX_BUFFER_SIZE];
static uint32_t g_rx_len = 0;
static uint8_t g_tx_buffer[TX_BUFFER_SIZE];
static uint32_t g_tx_len = 0;

#ifdef _WIN32
static CRITICAL_SECTION g_lock;
#else
static pthread_mutex_t g_lock = PTHREAD_MUTEX_INITIALIZER;
#endif

static volatile int g_running = 1;
static gh_protocol_handle_t* g_handle = NULL;

static void example_lock(void)
{
#ifdef _WIN32
    EnterCriticalSection(&g_lock);
#else
    pthread_mutex_lock(&g_lock);
#endif
}

static void example_unlock(void)
{
#ifdef _WIN32
    LeaveCriticalSection(&g_lock);
#else
    pthread_mutex_unlock(&g_lock);
#endif
}

static void example_delay(void)
{
#ifdef _WIN32
    Sleep(1);
#else
    usleep(1000);
#endif
}

static void example_send(void* data, int size)
{
    if (g_tx_len + size <= TX_BUFFER_SIZE) {
        memcpy(g_tx_buffer + g_tx_len, data, size);
        g_tx_len += size;
        printf("[TX] Sent %d bytes to serial port\n", size);
    } else {
        printf("[TX] Buffer overflow!\n");
    }
}

static void example_event_callback(uint8_t event_type, uint8_t* data, uint32_t size)
{
    printf("[Event] Type: 0x%02X, Size: %u\n", event_type, size);
}

static void example_frame_callback(data_frame_t* frame)
{
    printf("[Frame] Function ID: %d, Frame ID: %d, Timestamp: %d\n",
           frame->function_id, frame->frame_id, frame->timestamp);
    
    if (frame->p_rawdata && frame->rawdata_size > 0) {
        printf("[Frame] Raw data size: %d\n", frame->rawdata_size);
    }
    
    if (frame->p_gs_data && frame->gs_data_size > 0) {
        printf("[Frame] GS data size: %d\n", frame->gs_data_size);
    }
}

#ifdef _WIN32
static DWORD WINAPI rx_thread(LPVOID param)
#else
static void* rx_thread(void* param)
#endif
{
    (void)param;
    
    printf("[Thread] RX thread started\n");
    
    while (g_running) {
        example_lock();
        uint32_t len = g_rx_len;
        if (len > 0) {
            uint8_t temp[RX_BUFFER_SIZE];
            memcpy(temp, g_rx_buffer, len);
            g_rx_len = 0;
            example_unlock();
            
            printf("[RX] Processing %u bytes\n", len);
            gh_protocol_receive(g_handle, temp, len);
        } else {
            example_unlock();
            example_delay();
        }
    }
    
    printf("[Thread] RX thread stopped\n");
    return 0;
}

#ifdef _WIN32
static DWORD WINAPI tx_thread(LPVOID param)
#else
static void* tx_thread(void* param)
#endif
{
    (void)param;
    
    printf("[Thread] TX thread started\n");
    
    while (g_running) {
        example_lock();
        uint32_t len = g_tx_len;
        if (len > 0) {
            uint8_t temp[TX_BUFFER_SIZE];
            memcpy(temp, g_tx_buffer, len);
            g_tx_len = 0;
            example_unlock();
            
            printf("[Serial] Writing %u bytes to hardware\n", len);
        } else {
            example_unlock();
            example_delay();
        }
    }
    
    printf("[Thread] TX thread stopped\n");
    return 0;
}

void simulate_serial_receive(uint8_t* data, uint32_t len)
{
    example_lock();
    if (g_rx_len + len <= RX_BUFFER_SIZE) {
        memcpy(g_rx_buffer + g_rx_len, data, len);
        g_rx_len += len;
    }
    example_unlock();
}

void example_init(void)
{
#ifdef _WIN32
    InitializeCriticalSection(&g_lock);
#endif

    gh_protocol_config_t config = {
        .lock = example_lock,
        .unlock = example_unlock,
        .delay = example_delay,
        .send = example_send,
        .event_callback = example_event_callback,
        .frame_callback = example_frame_callback
    };

    g_handle = gh_protocol_create(&config);
    if (!g_handle) {
        printf("[Error] Failed to create protocol handle\n");
        return;
    }

    printf("[Init] Protocol initialized\n");
}

void example_start_threads(void)
{
#ifdef _WIN32
    CreateThread(NULL, 0, rx_thread, NULL, 0, NULL);
    CreateThread(NULL, 0, tx_thread, NULL, 0, NULL);
#else
    pthread_t tid;
    pthread_create(&tid, NULL, rx_thread, NULL);
    pthread_create(&tid, NULL, tx_thread, NULL);
#endif
}

void example_send_commands(void)
{
    printf("\n[CMD] Sending chip control command...\n");
    gh_protocol_chip_ctrl(g_handle, 1);
    
    example_delay();
    
    printf("[CMD] Setting timestamp...\n");
    gh_protocol_timestamp_set(g_handle, 12345678);
    
    example_delay();
    
    printf("[CMD] Getting version...\n");
    uint8_t version[64];
    uint16_t version_len = 0;
    gh_protocol_get_version(g_handle, 0, version, &version_len);
    if (version_len > 0) {
        printf("[CMD] Version: %.*s\n", version_len, version);
    }
    
    example_delay();
    
    printf("[CMD] Setting work mode...\n");
    gh_protocol_set_work_mode(g_handle, 1);
}

void example_cleanup(void)
{
    g_running = 0;
    
    if (g_handle) {
        gh_protocol_destroy(g_handle);
        g_handle = NULL;
    }

#ifdef _WIN32
    DeleteCriticalSection(&g_lock);
#endif

    printf("[Cleanup] Done\n");
}

int main(int argc, char* argv[])
{
    (void)argc;
    (void)argv;

    printf("=== GH Protocol Example ===\n\n");

    example_init();
    example_start_threads();

#ifdef _WIN32
    Sleep(100);
#else
    usleep(100000);
#endif

    example_send_commands();

    printf("\n[Main] Running for 5 seconds...\n");
#ifdef _WIN32
    Sleep(5000);
#else
    sleep(5);
#endif

    example_cleanup();

    printf("\n=== Example Complete ===\n");
    return 0;
}
