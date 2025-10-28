# \InviteApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**accept_invite**](InviteApi.md#accept_invite) | **POST** /v2/invite/accept | 
[**new_invite**](InviteApi.md#new_invite) | **POST** /v2/invite/new | 



## accept_invite

> models::AcceptInviteReturnType accept_invite(accept_invite_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**accept_invite_body** | [**AcceptInviteBody**](AcceptInviteBody.md) |  | [required] |

### Return type

[**models::AcceptInviteReturnType**](AcceptInviteReturnType.md)

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: application/json

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

