add_executable(gh_protocol_example main.c)

# Linux 需要显式链接数学库
if(UNIX AND NOT APPLE)
    target_link_libraries(gh_protocol_example gh_protocol m)
else()
    target_link_libraries(gh_protocol_example gh_protocol)
endif()
