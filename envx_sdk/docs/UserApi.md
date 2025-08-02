# \UserApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**get_many_users**](UserApi.md#get_many_users) | **GET** /v2/user/get-many | 
[**get_user_v2**](UserApi.md#get_user_v2) | **GET** /v2/user/{user_id} | 
[**new_user_v2**](UserApi.md#new_user_v2) | **POST** /v2/user/new | 



## get_many_users

> Vec<models::StrippedUser> get_many_users(uuid_colon_colon_uuid)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**uuid_colon_colon_uuid** | [**Vec<uuid::Uuid>**](uuid::Uuid.md) |  | [required] |

### Return type

[**Vec<models::StrippedUser>**](StrippedUser.md)

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## get_user_v2

> models::StrippedUser get_user_v2(user_id)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**user_id** | **uuid::Uuid** |  | [required] |

### Return type

[**models::StrippedUser**](StrippedUser.md)

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## new_user_v2

> String new_user_v2(new_user_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**new_user_body** | [**NewUserBody**](NewUserBody.md) |  | [required] |

### Return type

**String**

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

