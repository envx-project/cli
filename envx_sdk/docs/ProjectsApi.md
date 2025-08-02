# \ProjectsApi

All URIs are relative to *http://localhost*

Method | HTTP request | Description
------------- | ------------- | -------------
[**list_projects_v2**](ProjectsApi.md#list_projects_v2) | **GET** /v2/projects | 
[**new_project_v2**](ProjectsApi.md#new_project_v2) | **POST** /v2/projects/new | 



## list_projects_v2

> Vec<models::ListProjectsV2> list_projects_v2()


### Parameters

This endpoint does not need any parameter.

### Return type

[**Vec<models::ListProjectsV2>**](ListProjectsV2.md)

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: Not defined
- **Accept**: application/json

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)


## new_project_v2

> String new_project_v2(new_project_body)


### Parameters


Name | Type | Description  | Required | Notes
------------- | ------------- | ------------- | ------------- | -------------
**new_project_body** | [**NewProjectBody**](NewProjectBody.md) |  | [required] |

### Return type

**String**

### Authorization

[bearer](../README.md#bearer)

### HTTP request headers

- **Content-Type**: application/json
- **Accept**: text/plain

[[Back to top]](#) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to Model list]](../README.md#documentation-for-models) [[Back to README]](../README.md)

