# @file
# CMake package configuration for the Bluetooth HCI Rust library.
message(
  STATUS
  "Enabling BT HCI rust library components from ${CMAKE_CURRENT_LIST_DIR}"
)

set(CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo-target")

set(BT_HCI_RUST_DIR ${CMAKE_CURRENT_LIST_DIR})
set(BT_HCI_RUST_INCLUDE_DIRECTORIES ${BT_HCI_RUST_DIR})

if(HOST)
  set(FEATURES)
else(HOST)
  set(FEATURES
    --no-default-features
    --features libc,alloc,libc-panic,libc-alloc
  )
endif(HOST)

message(STATUS "Building BT HCI rust library with features: ${FEATURES}")

set(BT_HCI_RUST_LIB
  ${CARGO_TARGET_DIR}/${RUST_TARGET}/release/libbt_hci_rs.a
)

if(DEFINED BT_HCI_RUST_CONFIG_PATH)
  message(STATUS "Building BT HCI rust library with custom config ${BT_HCI_RUST_CONFIG_PATH}")
  set(RUST_BUILD_COMMAND
    ${CMAKE_COMMAND} -E env BT_HCI_RUST_CONFIG_PATH=${BT_HCI_RUST_CONFIG_PATH}
    cargo build
    --lib
    --target ${RUST_TARGET}
    --release
    --target-dir ${CARGO_TARGET_DIR}
    ${FEATURES}
  )
else()
  set(RUST_BUILD_COMMAND
    cargo build
    --lib
    --target ${RUST_TARGET}
    --release
    --target-dir ${CARGO_TARGET_DIR}
    ${FEATURES}
  )
endif()

add_custom_target(build_bt_hci_rust ALL
  COMMAND ${RUST_BUILD_COMMAND}
  WORKING_DIRECTORY ${BT_HCI_RUST_DIR}
  BYPRODUCTS
    ${BT_HCI_RUST_LIB}
  VERBATIM
  COMMENT "Building BT HCI rust library"
)

add_library(bt_hci_rust STATIC IMPORTED GLOBAL)
set_target_properties(bt_hci_rust PROPERTIES
  IMPORTED_LOCATION ${BT_HCI_RUST_LIB}
)

add_dependencies(bt_hci_rust build_bt_hci_rust)

set(BT_HCI_RUST_LINK_LIBRARIES bt_hci_rust)