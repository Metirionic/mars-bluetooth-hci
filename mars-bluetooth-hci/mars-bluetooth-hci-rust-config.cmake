# @file
# CMake package configuration for the Mars Bluetooth HCI Rust library.
message(
  STATUS
  "Enabling Mars BT HCI rust library components from ${CMAKE_CURRENT_LIST_DIR}"
)

set(CARGO_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo-target")

set(MARS_BT_HCI_RUST_DIR ${CMAKE_CURRENT_LIST_DIR})
set(MARS_BT_HCI_RUST_INCLUDE_DIRECTORIES ${MARS_BT_HCI_RUST_DIR})

if(HOST)
  set(FEATURES)
else(HOST)
  set(FEATURES
    --no-default-features
    --features libc,alloc,libc-panic,libc-alloc
  )
endif(HOST)

message(STATUS "Building Mars BT HCI rust library with features: ${FEATURES}")

set(MARS_BT_HCI_RUST_LIB
  ${CARGO_TARGET_DIR}/${RUST_TARGET}/release/libmars_bluetooth_hci.a
)

if(DEFINED MARS_BT_HCI_RUST_CONFIG_PATH)
  message(STATUS "Building Mars BT HCI rust library with custom config ${MARS_BT_HCI_RUST_CONFIG_PATH}")
  set(RUST_BUILD_COMMAND
    ${CMAKE_COMMAND} -E env MARS_BT_HCI_RUST_CONFIG_PATH=${MARS_BT_HCI_RUST_CONFIG_PATH}
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

add_custom_target(build_mars_bt_hci_rust ALL
  COMMAND ${RUST_BUILD_COMMAND}
  WORKING_DIRECTORY ${MARS_BT_HCI_RUST_DIR}
  BYPRODUCTS
    ${MARS_BT_HCI_RUST_LIB}
  VERBATIM
  COMMENT "Building Mars BT HCI rust library"
)

add_library(mars_bt_hci_rust STATIC IMPORTED GLOBAL)
set_target_properties(mars_bt_hci_rust PROPERTIES
  IMPORTED_LOCATION ${MARS_BT_HCI_RUST_LIB}
)

add_dependencies(mars_bt_hci_rust build_mars_bt_hci_rust)

set(MARS_BT_HCI_RUST_LINK_LIBRARIES mars_bt_hci_rust)