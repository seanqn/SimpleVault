cmake_minimum_required(VERSION 3.30)

project(SimpleVault VERSION 1.0 LANGUAGES CXX OBJCXX)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(Qt6_DIR "/Users/shonny/Qt/6.9.3/macos/lib/cmake/Qt6")
set(BUILD_WITH_QT6 ON CACHE BOOL "Force Qt6" FORCE)
set(Qt5Core_FOUND FALSE CACHE BOOL "" FORCE)
set(Qt5_FOUND FALSE CACHE BOOL "" FORCE)

# automation features required for Q_OBJECT functionality in test classes
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTOUIC ON)
set(CMAKE_AUTORCC ON)

find_package(Qt6 REQUIRED COMPONENTS Core Quick Sql Test)

list(APPEND CMAKE_PREFIX_PATH /Users/shonny/Qt/6.9.3/macos/lib/cmake)

qt_standard_project_setup(REQUIRES 6.8)

add_executable(SimpleVault
    app/main.cpp
)

# add_executable(Test_VaultManager
#     controller/tests_controller.cpp
#     controller/vaultmanager.cpp
#     services/credentialeditcache.cpp
# )

# QML_IMPORT_PATH("/Users/shonny/Qt/6.9.3/macos/qml")

set_source_files_properties(macos/statusitem_bridge.mm PROPERTIES
    COMPILE_FLAGS "-fobjc-arc"
)
target_sources(SimpleVault PRIVATE
    data/credentialtypes.h
    data/repositories/db_local.h data/repositories/db_local.cpp
    data/repository.h data/repository.cpp
    models/credentialmodel.h models/credentialmodel.cpp
    models/groupsmodel.h models/groupsmodel.cpp
    services/credentialeditcache.h services/credentialeditcache.cpp
    services/crypto/cryptoservicer.h services/crypto/cryptoservicer.cpp
    macos/nsstatus_bridge.mm
)

# after debug is finished, review and keep only necessary sources
qt_add_qml_module(SimpleVault
    URI SimpleVault
    VERSION 1.0
    QML_FILES
        views/Main.qml
        views/home/Home.qml
        views/auth/AuthStack.qml
        views/auth/AuthForm.qml
        views/auth/RegisterForm.qml
        views/home/components/HomePane.qml
        tests/groups_vaultcontent_tests.qml
    SOURCES
        services/vaultsecurityservicer.h services/vaultsecurityservicer.cpp
        controller/vaultmanager.h controller/vaultmanager.cpp
        SOURCES macos/nsstatus_bridge.h
)

# Qt for iOS sets MACOSX_BUNDLE_GUI_IDENTIFIER automatically since Qt 6.1.
# If you are developing for iOS or macOS you should consider setting an
# explicit, fixed bundle identifier manually though.
set_target_properties(SimpleVault PROPERTIES
    MACOSX_BUNDLE_INFO_PLIST ${CMAKE_CURRENT_SOURCE_DIR}/config/Info.plist
    MACOSX_BUNDLE_BUNDLE_VERSION ${PROJECT_VERSION}
    MACOSX_BUNDLE_SHORT_VERSION_STRING ${PROJECT_VERSION_MAJOR}.${PROJECT_VERSION_MINOR}
    MACOSX_BUNDLE TRUE
)

# set(MY_ENTITLEMENTS ${CMAKE_CURRENT_SOURCE_DIR}/config/entitlements.plist)

# add_custom_command(TARGET SimpleVault POST_BUILD
#     COMMAND codesign --force --deep --sign --entitlements ${MY_ENTITLEMENTS} $<TARGET_BUNDLE_DIR:SimpleVault>
#     COMMENT "Applying Ad-Hoc signing and keychain entitlements"
# )

# QT KEYCHAIN
include(FetchContent)
FetchContent_Declare(
    qtkeychain
    GIT_REPOSITORY https://github.com/frankosterfeld/qtkeychain
    GIT_TAG ad7344c45a86a4f66cbafc4b081b5f7b876cb0b7 # v0.15.0
)
FetchContent_MakeAvailable(qtkeychain)
#

# libsodium
find_path(SODIUM_INCLUDE_DIR
    NAMES sodium.h
    PATHS /usr/local/opt/libsodium/include /usr/local/include
)

find_library(SODIUM_LIB_STATIC
    NAMES libsodium.a sodium
    PATHS /usr/local/opt/libsodium/lib /usr/local/lib
)

add_library(sodium::sodium STATIC IMPORTED GLOBAL)
set_target_properties(sodium::sodium PROPERTIES
    IMPORTED_LOCATION "${SODIUM_LIB_STATIC}"
    INTERFACE_INCLUDE_DIRECTORIES "${SODIUM_INCLUDE_DIR}"
)
#

target_include_directories(SimpleVault PRIVATE
    ${qtkeychain_BINARY_DIR}
    ${qtkeychain_SOURCE_DIR}
)

target_link_libraries(SimpleVault PRIVATE
    Qt6::Core
    Qt6::Quick
    Qt6::Sql
    Qt6::Test
    qt6keychain
    "-framework AppKit"
    "-framework Cocoa"
    sodium::sodium
)
#

enable_testing()
add_executable(Test_CryptoServicer
    services/crypto/tests_cryptoservicer.cpp

    # addt
    services/crypto/cryptoservicer.h services/crypto/cryptoservicer.cpp
)
target_link_libraries(Test_CryptoServicer PRIVATE
    Qt6::Core
    Qt6::Test
    sodium::sodium
)
add_test(NAME Test_CryptoServicer COMMAND Test_CryptoServicer --verbose)

# add_test(NAME Test_VaultManager COMMAND Test_VaultManager)

include(GNUInstallDirs)

install(TARGETS SimpleVault
    BUNDLE DESTINATION .
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
)
