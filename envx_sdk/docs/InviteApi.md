# \InviteApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**accept_invite**](InviteApi.md#accept_invite) | **POST** /v2/invite/accept/{invite_code} | 
[**new_invite**](InviteApi.md#new_invite) | **POST** /v2/invite/new | 



## accept_invite

> String accept_invite(invite_code, accept_invite_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**invite_code** | **uuid::Uuid** |  | [required] |
**accept_invite_body** | [**AcceptInviteBody**](AcceptInviteBody.md) |  | [required] |

### Return type

**String**

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## new_invite

> models::InviteResponse new_invite(invite_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**invite_body** | [**InviteBody**](InviteBody.md) |  | [required] |

### Return type

[**models::InviteResponse**](InviteResponse.md)

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

