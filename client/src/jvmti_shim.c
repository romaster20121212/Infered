#include <jni.h>
#include <jvmti.h>
#include <string.h>
#include <stdint.h>

#ifdef _WIN32
#define DARK_EXPORT __declspec(dllexport)
#else
#define DARK_EXPORT __attribute__((visibility("default")))
#endif

typedef int (*dark_class_file_load_callback)(
    const char *class_name,
    jint class_name_length,
    unsigned char class_being_redefined_nonnull,
    jint class_data_len,
    const unsigned char *class_data,
    unsigned char **replacement,
    jint *replacement_len);

extern void dark_free_transformed_bytes(unsigned char *ptr, jint len);

static jvmtiEnv *dark_jvmti = NULL;
static dark_class_file_load_callback dark_class_callback = NULL;
static jvmtiEnv *dark_capability_env = NULL;

 
DARK_EXPORT int dark_jvmti_diagnose(
    JavaVM *vm,
    int *statuses,
    int status_count,
    int *values,
    int value_count) {
    if (vm == NULL || statuses == NULL || status_count < 9 ||
        values == NULL || value_count < 14) {
        return -1;
    }
    for (int i = 0; i < 9; ++i) statuses[i] = -1;
    for (int i = 0; i < 14; ++i) values[i] = -1;

    jvmtiEnv *jvmti = NULL;
    jint result = (*vm)->GetEnv(vm, (void **)&jvmti, JVMTI_VERSION_1_2);
    statuses[0] = (int)result;
    if (result != JNI_OK || jvmti == NULL) return (int)result;

    jint version = 0;
    result = (*jvmti)->GetVersionNumber(jvmti, &version);
    statuses[1] = (int)result;
    if (result != JVMTI_ERROR_NONE) return (int)result;
    values[0] = (int)version;

    jvmtiPhase phase = (jvmtiPhase)0;
    result = (*jvmti)->GetPhase(jvmti, &phase);
    statuses[2] = (int)result;
    if (result != JVMTI_ERROR_NONE) return (int)result;
    values[1] = (int)phase;

    jvmtiCapabilities current;
    memset(&current, 0, sizeof(current));
    result = (*jvmti)->GetCapabilities(jvmti, &current);
    statuses[3] = (int)result;
    if (result != JVMTI_ERROR_NONE) return (int)result;
    values[2] = current.can_generate_method_exit_events ? 1 : 0;
    values[3] = current.can_retransform_classes ? 1 : 0;
    values[4] = current.can_generate_all_class_hook_events ? 1 : 0;

    jvmtiCapabilities potential;
    memset(&potential, 0, sizeof(potential));
    result = (*jvmti)->GetPotentialCapabilities(jvmti, &potential);
    statuses[4] = (int)result;
    if (result != JVMTI_ERROR_NONE) return (int)result;
    values[5] = potential.can_generate_method_exit_events ? 1 : 0;
    values[6] = potential.can_retransform_classes ? 1 : 0;
    values[7] = potential.can_generate_all_class_hook_events ? 1 : 0;

    jvmtiCapabilities request;
    memset(&request, 0, sizeof(request));
    request.can_generate_method_exit_events = 1;
    statuses[5] = (int)(*jvmti)->AddCapabilities(jvmti, &request);

    memset(&current, 0, sizeof(current));
    statuses[6] = (int)(*jvmti)->GetCapabilities(jvmti, &current);
    if (statuses[6] == JVMTI_ERROR_NONE) {
        values[8] = current.can_generate_method_exit_events ? 1 : 0;
        values[9] = current.can_retransform_classes ? 1 : 0;
        values[10] = current.can_generate_all_class_hook_events ? 1 : 0;
    }

    memset(&request, 0, sizeof(request));
    request.can_retransform_classes = 1;
    statuses[7] = (int)(*jvmti)->AddCapabilities(jvmti, &request);

    memset(&current, 0, sizeof(current));
    statuses[8] = (int)(*jvmti)->GetCapabilities(jvmti, &current);
    if (statuses[8] == JVMTI_ERROR_NONE) {
        values[11] = current.can_generate_method_exit_events ? 1 : 0;
        values[12] = current.can_retransform_classes ? 1 : 0;
        values[13] = current.can_generate_all_class_hook_events ? 1 : 0;
    }
    return statuses[0] != JNI_OK ? statuses[0] :
           (statuses[1] != 0 ? statuses[1] :
           (statuses[2] != 0 ? statuses[2] :
           (statuses[3] != 0 ? statuses[3] :
           (statuses[4] != 0 ? statuses[4] :
           (statuses[5] != 0 ? statuses[5] :
           (statuses[6] != 0 ? statuses[6] :
           (statuses[7] != 0 ? statuses[7] : statuses[8])))))));
}

static void JNICALL dark_class_file_load_hook(
    jvmtiEnv *jvmti,
    JNIEnv *env,
    jclass class_being_redefined,
    jobject loader,
    const char *name,
    jobject protection_domain,
    jint class_data_len,
    const unsigned char *class_data,
    jint *new_class_data_len,
    unsigned char **new_class_data) {
    (void)jvmti;
    (void)env;
    (void)loader;
    (void)protection_domain;
    (void)class_data_len;
    (void)class_data;
    if (new_class_data_len != NULL) {
        *new_class_data_len = 0;
    }
    if (new_class_data != NULL) {
        *new_class_data = NULL;
    }

    if (dark_class_callback != NULL) {
        unsigned char *replacement = NULL;
        jint replacement_len = 0;
        int transformed = dark_class_callback(
            name,
            name == NULL ? 0 : (jint)strlen(name),
            class_being_redefined != NULL ? 1 : 0,
            class_data_len,
            class_data,
            &replacement,
            &replacement_len);
        if (transformed && replacement != NULL && replacement_len > 0 &&
            new_class_data_len != NULL && new_class_data != NULL) {
            unsigned char *allocated = NULL;
            jvmtiError result = (*jvmti)->Allocate(jvmti, replacement_len, &allocated);
            if (result == JVMTI_ERROR_NONE && allocated != NULL) {
                memcpy(allocated, replacement, (size_t)replacement_len);
                *new_class_data_len = replacement_len;
                *new_class_data = allocated;
            }
        }
        if (replacement != NULL && replacement_len > 0) {
            dark_free_transformed_bytes(replacement, replacement_len);
        }
    }
}

 






DARK_EXPORT int dark_jvmti_class_hook_diagnose(
    JavaVM *vm,
    dark_class_file_load_callback callback,
    int *statuses,
    int status_count,
    int *values,
    int value_count,
    uintptr_t *pointers,
    int pointer_count) {
    if (vm == NULL || callback == NULL || statuses == NULL || status_count < 7 ||
        values == NULL || value_count < 4 || pointers == NULL || pointer_count < 10) {
        return -1;
    }

    for (int i = 0; i < 7; ++i) {
        statuses[i] = -1;
    }
    for (int i = 0; i < 4; ++i) {
        values[i] = -1;
    }
    for (int i = 0; i < 10; ++i) {
        pointers[i] = 0;
    }

    jvmtiEnv *jvmti = NULL;
    jint result = (*vm)->GetEnv(vm, (void **)&jvmti, JVMTI_VERSION_1_2);
    statuses[0] = (int)result;
    if (result != JNI_OK || jvmti == NULL) {
        return (int)result;
    }
    dark_capability_env = jvmti;
    pointers[0] = (uintptr_t)jvmti;  
    pointers[7] = (uintptr_t)(*jvmti);  
    pointers[8] = (uintptr_t)(*jvmti)->AddCapabilities;  
    pointers[9] = (uintptr_t)(*jvmti)->RetransformClasses;  

    jvmtiCapabilities current;
    memset(&current, 0, sizeof(current));
    result = (*jvmti)->GetCapabilities(jvmti, &current);
    statuses[1] = (int)result;
    if (result != JVMTI_ERROR_NONE) {
        return (int)result;
    }
    values[0] = current.can_retransform_classes ? 1 : 0;
    values[1] = current.can_generate_all_class_hook_events ? 1 : 0;

    jvmtiCapabilities request;
    memset(&request, 0, sizeof(request));
    request.can_retransform_classes = 1;
    result = (*jvmti)->AddCapabilities(jvmti, &request);
    statuses[2] = (int)result;
    pointers[2] = (uintptr_t)jvmti;  

    memset(&request, 0, sizeof(request));
    request.can_generate_all_class_hook_events = 1;
    result = (*jvmti)->AddCapabilities(jvmti, &request);
    statuses[3] = (int)result;
    pointers[3] = (uintptr_t)jvmti;  

    memset(&current, 0, sizeof(current));
    result = (*jvmti)->GetCapabilities(jvmti, &current);
    statuses[4] = (int)result;
    pointers[4] = (uintptr_t)jvmti;  
    if (result == JVMTI_ERROR_NONE) {
        values[2] = current.can_retransform_classes ? 1 : 0;
        values[3] = current.can_generate_all_class_hook_events ? 1 : 0;
    }

    memset(&current, 0, sizeof(current));
    current.can_retransform_classes = 0;  

    jvmtiEventCallbacks callbacks;
    memset(&callbacks, 0, sizeof(callbacks));
    callbacks.ClassFileLoadHook = (jvmtiEventClassFileLoadHook)dark_class_file_load_hook;
    result = (*jvmti)->SetEventCallbacks(
        jvmti,
        &callbacks,
        (jint)sizeof(callbacks));
    statuses[5] = (int)result;
    pointers[5] = (uintptr_t)jvmti;  
    if (result != JVMTI_ERROR_NONE) {
        return (int)result;
    }

     
    dark_jvmti = jvmti;
    dark_class_callback = callback;
    pointers[6] = (uintptr_t)jvmti;  
    result = (*jvmti)->SetEventNotificationMode(
        jvmti,
        JVMTI_ENABLE,
        JVMTI_EVENT_CLASS_FILE_LOAD_HOOK,
        NULL);
    statuses[6] = (int)result;
    if (result != JVMTI_ERROR_NONE) {
        dark_class_callback = NULL;
        dark_jvmti = NULL;
        return (int)result;
    }
    return JVMTI_ERROR_NONE;
}

DARK_EXPORT int dark_jvmti_retransform_diagnose(
    jclass target_class,
    int *statuses,
    int status_count,
    int *values,
    int value_count,
    uintptr_t *pointers,
    int pointer_count) {
    if (target_class == NULL || statuses == NULL || status_count < 2 ||
        values == NULL || value_count < 3 || pointers == NULL || pointer_count < 4) {
        return -1;
    }
    statuses[0] = statuses[1] = -1;
    values[0] = -1;
    for (int i = 0; i < 4; ++i) pointers[i] = 0;

    jvmtiEnv *jvmti = dark_capability_env;
    if (jvmti == NULL) {
        return JVMTI_ERROR_NULL_POINTER;
    }
    pointers[0] = (uintptr_t)jvmti;  
    pointers[1] = (uintptr_t)jvmti;  
    pointers[2] = (uintptr_t)(*jvmti);  
    pointers[3] = (uintptr_t)(*jvmti)->RetransformClasses;  

    jboolean modifiable = JNI_FALSE;
    jint result = (*jvmti)->IsModifiableClass(jvmti, target_class, &modifiable);
    statuses[0] = (int)result;
    if (result != JVMTI_ERROR_NONE) {
        return (int)result;
    }
    values[0] = modifiable ? 1 : 0;
    jvmtiCapabilities before_retransform;
    memset(&before_retransform, 0, sizeof(before_retransform));
    result = (*jvmti)->GetCapabilities(jvmti, &before_retransform);
    if (result != JVMTI_ERROR_NONE) {
        return (int)result;
    }
    values[1] = before_retransform.can_retransform_classes ? 1 : 0;
    values[2] = before_retransform.can_generate_all_class_hook_events ? 1 : 0;
    if (!modifiable) {
        return JVMTI_ERROR_UNMODIFIABLE_CLASS;
    }

    jclass classes[1] = { target_class };
    result = (*jvmti)->RetransformClasses(jvmti, (jint)1, classes);
    statuses[1] = (int)result;
    return (int)result;
}

 





DARK_EXPORT const char *dark_jvmti_status_name(int status) {
    switch (status) {
    case JVMTI_ERROR_NONE: return "JVMTI_ERROR_NONE";
    case JVMTI_ERROR_INVALID_THREAD: return "JVMTI_ERROR_INVALID_THREAD";
    case JVMTI_ERROR_INVALID_THREAD_GROUP: return "JVMTI_ERROR_INVALID_THREAD_GROUP";
    case JVMTI_ERROR_INVALID_PRIORITY: return "JVMTI_ERROR_INVALID_PRIORITY";
    case JVMTI_ERROR_THREAD_NOT_SUSPENDED: return "JVMTI_ERROR_THREAD_NOT_SUSPENDED";
    case JVMTI_ERROR_THREAD_SUSPENDED: return "JVMTI_ERROR_THREAD_SUSPENDED";
    case JVMTI_ERROR_THREAD_NOT_ALIVE: return "JVMTI_ERROR_THREAD_NOT_ALIVE";
    case JVMTI_ERROR_INVALID_OBJECT: return "JVMTI_ERROR_INVALID_OBJECT";
    case JVMTI_ERROR_INVALID_CLASS: return "JVMTI_ERROR_INVALID_CLASS";
    case JVMTI_ERROR_CLASS_NOT_PREPARED: return "JVMTI_ERROR_CLASS_NOT_PREPARED";
    case JVMTI_ERROR_INVALID_METHODID: return "JVMTI_ERROR_INVALID_METHODID";
    case JVMTI_ERROR_INVALID_LOCATION: return "JVMTI_ERROR_INVALID_LOCATION";
    case JVMTI_ERROR_INVALID_FIELDID: return "JVMTI_ERROR_INVALID_FIELDID";
    case JVMTI_ERROR_INVALID_MODULE: return "JVMTI_ERROR_INVALID_MODULE";
    case JVMTI_ERROR_NO_MORE_FRAMES: return "JVMTI_ERROR_NO_MORE_FRAMES";
    case JVMTI_ERROR_OPAQUE_FRAME: return "JVMTI_ERROR_OPAQUE_FRAME";
    case JVMTI_ERROR_TYPE_MISMATCH: return "JVMTI_ERROR_TYPE_MISMATCH";
    case JVMTI_ERROR_INVALID_SLOT: return "JVMTI_ERROR_INVALID_SLOT";
    case JVMTI_ERROR_DUPLICATE: return "JVMTI_ERROR_DUPLICATE";
    case JVMTI_ERROR_NOT_FOUND: return "JVMTI_ERROR_NOT_FOUND";
    case JVMTI_ERROR_INVALID_MONITOR: return "JVMTI_ERROR_INVALID_MONITOR";
    case JVMTI_ERROR_NOT_MONITOR_OWNER: return "JVMTI_ERROR_NOT_MONITOR_OWNER";
    case JVMTI_ERROR_INTERRUPT: return "JVMTI_ERROR_INTERRUPT";
    case JVMTI_ERROR_INVALID_CLASS_FORMAT: return "JVMTI_ERROR_INVALID_CLASS_FORMAT";
    case JVMTI_ERROR_CIRCULAR_CLASS_DEFINITION: return "JVMTI_ERROR_CIRCULAR_CLASS_DEFINITION";
    case JVMTI_ERROR_FAILS_VERIFICATION: return "JVMTI_ERROR_FAILS_VERIFICATION";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_METHOD_ADDED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_METHOD_ADDED";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_SCHEMA_CHANGED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_SCHEMA_CHANGED";
    case JVMTI_ERROR_INVALID_TYPESTATE: return "JVMTI_ERROR_INVALID_TYPESTATE";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_HIERARCHY_CHANGED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_HIERARCHY_CHANGED";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_METHOD_DELETED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_METHOD_DELETED";
    case JVMTI_ERROR_UNSUPPORTED_VERSION: return "JVMTI_ERROR_UNSUPPORTED_VERSION";
    case JVMTI_ERROR_NAMES_DONT_MATCH: return "JVMTI_ERROR_NAMES_DONT_MATCH";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_CLASS_MODIFIERS_CHANGED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_CLASS_MODIFIERS_CHANGED";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_METHOD_MODIFIERS_CHANGED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_METHOD_MODIFIERS_CHANGED";
    case JVMTI_ERROR_UNSUPPORTED_REDEFINITION_CLASS_ATTRIBUTE_CHANGED:
        return "JVMTI_ERROR_UNSUPPORTED_REDEFINITION_CLASS_ATTRIBUTE_CHANGED";
    case JVMTI_ERROR_UNSUPPORTED_OPERATION: return "JVMTI_ERROR_UNSUPPORTED_OPERATION";
    case JVMTI_ERROR_UNMODIFIABLE_CLASS: return "JVMTI_ERROR_UNMODIFIABLE_CLASS";
    case JVMTI_ERROR_UNMODIFIABLE_MODULE: return "JVMTI_ERROR_UNMODIFIABLE_MODULE";
    case JVMTI_ERROR_NOT_AVAILABLE: return "JVMTI_ERROR_NOT_AVAILABLE";
    case JVMTI_ERROR_MUST_POSSESS_CAPABILITY: return "JVMTI_ERROR_MUST_POSSESS_CAPABILITY";
    case JVMTI_ERROR_NULL_POINTER: return "JVMTI_ERROR_NULL_POINTER";
    case JVMTI_ERROR_ABSENT_INFORMATION: return "JVMTI_ERROR_ABSENT_INFORMATION";
    case JVMTI_ERROR_INVALID_EVENT_TYPE: return "JVMTI_ERROR_INVALID_EVENT_TYPE";
    case JVMTI_ERROR_ILLEGAL_ARGUMENT: return "JVMTI_ERROR_ILLEGAL_ARGUMENT";
    case JVMTI_ERROR_NATIVE_METHOD: return "JVMTI_ERROR_NATIVE_METHOD";
    case JVMTI_ERROR_CLASS_LOADER_UNSUPPORTED: return "JVMTI_ERROR_CLASS_LOADER_UNSUPPORTED";
    case JVMTI_ERROR_OUT_OF_MEMORY: return "JVMTI_ERROR_OUT_OF_MEMORY";
    case JVMTI_ERROR_ACCESS_DENIED: return "JVMTI_ERROR_ACCESS_DENIED";
    case JVMTI_ERROR_WRONG_PHASE: return "JVMTI_ERROR_WRONG_PHASE";
    case JVMTI_ERROR_INTERNAL: return "JVMTI_ERROR_INTERNAL";
    case JVMTI_ERROR_UNATTACHED_THREAD: return "JVMTI_ERROR_UNATTACHED_THREAD";
    case JVMTI_ERROR_INVALID_ENVIRONMENT: return "JVMTI_ERROR_INVALID_ENVIRONMENT";
    default: return "JVMTI_ERROR_UNKNOWN";
    }
}

DARK_EXPORT void dark_jvmti_class_hook_uninstall(void) {
    if (dark_jvmti != NULL) {
        (*dark_jvmti)->SetEventNotificationMode(
            dark_jvmti,
            JVMTI_DISABLE,
            JVMTI_EVENT_CLASS_FILE_LOAD_HOOK,
            NULL);
        (*dark_jvmti)->SetEventCallbacks(dark_jvmti, NULL, 0);
    }
    dark_class_callback = NULL;
    dark_jvmti = NULL;
    dark_capability_env = NULL;
}
