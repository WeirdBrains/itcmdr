// This is a generated file - do not edit.
//
// Generated from itcmdr.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports
// ignore_for_file: unused_import

import 'dart:convert' as $convert;
import 'dart:core' as $core;
import 'dart:typed_data' as $typed_data;

@$core.Deprecated('Use rollupDescriptor instead')
const Rollup$json = {
  '1': 'Rollup',
  '2': [
    {'1': 'ROLLUP_DAILY', '2': 0},
    {'1': 'ROLLUP_WEEKLY', '2': 1},
    {'1': 'ROLLUP_BLOCK4', '2': 2},
    {'1': 'ROLLUP_YEARLY', '2': 4},
  ],
  '4': [
    {'1': 3, '2': 3},
  ],
};

/// Descriptor for `Rollup`. Decode as a `google.protobuf.EnumDescriptorProto`.
final $typed_data.Uint8List rollupDescriptor = $convert.base64Decode(
    'CgZSb2xsdXASEAoMUk9MTFVQX0RBSUxZEAASEQoNUk9MTFVQX1dFRUtMWRABEhEKDVJPTExVUF'
    '9CTE9DSzQQAhIRCg1ST0xMVVBfWUVBUkxZEAQiBAgDEAM=');

@$core.Deprecated('Use machineDescriptor instead')
const Machine$json = {
  '1': 'Machine',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {'1': 'status', '3': 2, '4': 1, '5': 9, '10': 'status'},
    {'1': 'open_incidents', '3': 3, '4': 1, '5': 5, '10': 'openIncidents'},
    {'1': 'last_run_ts', '3': 4, '4': 1, '5': 3, '10': 'lastRunTs'},
  ],
};

/// Descriptor for `Machine`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List machineDescriptor = $convert.base64Decode(
    'CgdNYWNoaW5lEhIKBG5hbWUYASABKAlSBG5hbWUSFgoGc3RhdHVzGAIgASgJUgZzdGF0dXMSJQ'
    'oOb3Blbl9pbmNpZGVudHMYAyABKAVSDW9wZW5JbmNpZGVudHMSHgoLbGFzdF9ydW5fdHMYBCAB'
    'KANSCWxhc3RSdW5Ucw==');

@$core.Deprecated('Use listMachinesRequestDescriptor instead')
const ListMachinesRequest$json = {
  '1': 'ListMachinesRequest',
};

/// Descriptor for `ListMachinesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listMachinesRequestDescriptor =
    $convert.base64Decode('ChNMaXN0TWFjaGluZXNSZXF1ZXN0');

@$core.Deprecated('Use listMachinesResponseDescriptor instead')
const ListMachinesResponse$json = {
  '1': 'ListMachinesResponse',
  '2': [
    {
      '1': 'machines',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.Machine',
      '10': 'machines'
    },
  ],
};

/// Descriptor for `ListMachinesResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listMachinesResponseDescriptor = $convert.base64Decode(
    'ChRMaXN0TWFjaGluZXNSZXNwb25zZRIrCghtYWNoaW5lcxgBIAMoCzIPLml0Y21kci5NYWNoaW'
    '5lUghtYWNoaW5lcw==');

@$core.Deprecated('Use triggerRunsRequestDescriptor instead')
const TriggerRunsRequest$json = {
  '1': 'TriggerRunsRequest',
  '2': [
    {'1': 'machines', '3': 1, '4': 3, '5': 9, '10': 'machines'},
    {'1': 'report_only', '3': 2, '4': 1, '5': 8, '10': 'reportOnly'},
    {'1': 'repair', '3': 3, '4': 1, '5': 8, '10': 'repair'},
    {'1': 'force', '3': 4, '4': 1, '5': 8, '10': 'force'},
    {'1': 'categories', '3': 5, '4': 3, '5': 9, '10': 'categories'},
    {'1': 'plugins', '3': 6, '4': 3, '5': 9, '10': 'plugins'},
  ],
};

/// Descriptor for `TriggerRunsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List triggerRunsRequestDescriptor = $convert.base64Decode(
    'ChJUcmlnZ2VyUnVuc1JlcXVlc3QSGgoIbWFjaGluZXMYASADKAlSCG1hY2hpbmVzEh8KC3JlcG'
    '9ydF9vbmx5GAIgASgIUgpyZXBvcnRPbmx5EhYKBnJlcGFpchgDIAEoCFIGcmVwYWlyEhQKBWZv'
    'cmNlGAQgASgIUgVmb3JjZRIeCgpjYXRlZ29yaWVzGAUgAygJUgpjYXRlZ29yaWVzEhgKB3BsdW'
    'dpbnMYBiADKAlSB3BsdWdpbnM=');

@$core.Deprecated('Use triggerRunsResponseDescriptor instead')
const TriggerRunsResponse$json = {
  '1': 'TriggerRunsResponse',
  '2': [
    {'1': 'request_id', '3': 1, '4': 1, '5': 9, '10': 'requestId'},
  ],
};

/// Descriptor for `TriggerRunsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List triggerRunsResponseDescriptor = $convert.base64Decode(
    'ChNUcmlnZ2VyUnVuc1Jlc3BvbnNlEh0KCnJlcXVlc3RfaWQYASABKAlSCXJlcXVlc3RJZA==');

@$core.Deprecated('Use getRequestStatusRequestDescriptor instead')
const GetRequestStatusRequest$json = {
  '1': 'GetRequestStatusRequest',
  '2': [
    {'1': 'request_id', '3': 1, '4': 1, '5': 9, '10': 'requestId'},
  ],
};

/// Descriptor for `GetRequestStatusRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getRequestStatusRequestDescriptor =
    $convert.base64Decode(
        'ChdHZXRSZXF1ZXN0U3RhdHVzUmVxdWVzdBIdCgpyZXF1ZXN0X2lkGAEgASgJUglyZXF1ZXN0SW'
        'Q=');

@$core.Deprecated('Use requestStatusDescriptor instead')
const RequestStatus$json = {
  '1': 'RequestStatus',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.itcmdr.RequestStatus.Status',
      '10': 'status'
    },
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
  ],
  '4': [RequestStatus_Status$json],
};

@$core.Deprecated('Use requestStatusDescriptor instead')
const RequestStatus_Status$json = {
  '1': 'Status',
  '2': [
    {'1': 'UNKNOWN', '2': 0},
    {'1': 'QUEUED', '2': 1},
    {'1': 'RUNNING', '2': 2},
    {'1': 'DONE', '2': 3},
    {'1': 'FAILED', '2': 4},
    {'1': 'TIMED_OUT', '2': 5},
  ],
};

/// Descriptor for `RequestStatus`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List requestStatusDescriptor = $convert.base64Decode(
    'Cg1SZXF1ZXN0U3RhdHVzEjQKBnN0YXR1cxgBIAEoDjIcLml0Y21kci5SZXF1ZXN0U3RhdHVzLl'
    'N0YXR1c1IGc3RhdHVzEhgKB21lc3NhZ2UYAiABKAlSB21lc3NhZ2UiUwoGU3RhdHVzEgsKB1VO'
    'S05PV04QABIKCgZRVUVVRUQQARILCgdSVU5OSU5HEAISCAoERE9ORRADEgoKBkZBSUxFRBAEEg'
    '0KCVRJTUVEX09VVBAF');

@$core.Deprecated('Use getRequestStatusResponseDescriptor instead')
const GetRequestStatusResponse$json = {
  '1': 'GetRequestStatusResponse',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 14,
      '6': '.itcmdr.RequestStatus.Status',
      '10': 'status'
    },
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
  ],
};

/// Descriptor for `GetRequestStatusResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getRequestStatusResponseDescriptor =
    $convert.base64Decode(
        'ChhHZXRSZXF1ZXN0U3RhdHVzUmVzcG9uc2USNAoGc3RhdHVzGAEgASgOMhwuaXRjbWRyLlJlcX'
        'Vlc3RTdGF0dXMuU3RhdHVzUgZzdGF0dXMSGAoHbWVzc2FnZRgCIAEoCVIHbWVzc2FnZQ==');

@$core.Deprecated('Use getMetricsRequestDescriptor instead')
const GetMetricsRequest$json = {
  '1': 'GetMetricsRequest',
  '2': [
    {'1': 'window_days', '3': 1, '4': 1, '5': 5, '10': 'windowDays'},
    {
      '1': 'rollup',
      '3': 2,
      '4': 1,
      '5': 14,
      '6': '.itcmdr.Rollup',
      '10': 'rollup'
    },
  ],
};

/// Descriptor for `GetMetricsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getMetricsRequestDescriptor = $convert.base64Decode(
    'ChFHZXRNZXRyaWNzUmVxdWVzdBIfCgt3aW5kb3dfZGF5cxgBIAEoBVIKd2luZG93RGF5cxImCg'
    'Zyb2xsdXAYAiABKA4yDi5pdGNtZHIuUm9sbHVwUgZyb2xsdXA=');

@$core.Deprecated('Use metricPointDescriptor instead')
const MetricPoint$json = {
  '1': 'MetricPoint',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'ts', '3': 2, '4': 1, '5': 3, '10': 'ts'},
    {'1': 'value', '3': 3, '4': 1, '5': 1, '10': 'value'},
  ],
};

/// Descriptor for `MetricPoint`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List metricPointDescriptor = $convert.base64Decode(
    'CgtNZXRyaWNQb2ludBIQCgNrZXkYASABKAlSA2tleRIOCgJ0cxgCIAEoA1ICdHMSFAoFdmFsdW'
    'UYAyABKAFSBXZhbHVl');

@$core.Deprecated('Use getMetricsResponseDescriptor instead')
const GetMetricsResponse$json = {
  '1': 'GetMetricsResponse',
  '2': [
    {
      '1': 'points',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.MetricPoint',
      '10': 'points'
    },
  ],
};

/// Descriptor for `GetMetricsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getMetricsResponseDescriptor = $convert.base64Decode(
    'ChJHZXRNZXRyaWNzUmVzcG9uc2USKwoGcG9pbnRzGAEgAygLMhMuaXRjbWRyLk1ldHJpY1BvaW'
    '50UgZwb2ludHM=');

@$core.Deprecated('Use exceptionScopeDescriptor instead')
const ExceptionScope$json = {
  '1': 'ExceptionScope',
  '2': [
    {'1': 'machines', '3': 1, '4': 3, '5': 9, '10': 'machines'},
    {'1': 'role', '3': 2, '4': 1, '5': 9, '10': 'role'},
  ],
};

/// Descriptor for `ExceptionScope`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List exceptionScopeDescriptor = $convert.base64Decode(
    'Cg5FeGNlcHRpb25TY29wZRIaCghtYWNoaW5lcxgBIAMoCVIIbWFjaGluZXMSEgoEcm9sZRgCIA'
    'EoCVIEcm9sZQ==');

@$core.Deprecated('Use exceptionRuleDescriptor instead')
const ExceptionRule$json = {
  '1': 'ExceptionRule',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {
      '1': 'scope',
      '3': 2,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.ExceptionScope',
      '10': 'scope'
    },
    {'1': 'check_id', '3': 3, '4': 1, '5': 9, '10': 'checkId'},
    {'1': 'check_pattern', '3': 4, '4': 1, '5': 9, '10': 'checkPattern'},
    {'1': 'action', '3': 5, '4': 1, '5': 9, '10': 'action'},
    {'1': 'reason', '3': 6, '4': 1, '5': 9, '10': 'reason'},
    {'1': 'expires', '3': 7, '4': 1, '5': 9, '10': 'expires'},
    {'1': 'active', '3': 8, '4': 1, '5': 8, '10': 'active'},
  ],
};

/// Descriptor for `ExceptionRule`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List exceptionRuleDescriptor = $convert.base64Decode(
    'Cg1FeGNlcHRpb25SdWxlEg4KAmlkGAEgASgJUgJpZBIsCgVzY29wZRgCIAEoCzIWLml0Y21kci'
    '5FeGNlcHRpb25TY29wZVIFc2NvcGUSGQoIY2hlY2tfaWQYAyABKAlSB2NoZWNrSWQSIwoNY2hl'
    'Y2tfcGF0dGVybhgEIAEoCVIMY2hlY2tQYXR0ZXJuEhYKBmFjdGlvbhgFIAEoCVIGYWN0aW9uEh'
    'YKBnJlYXNvbhgGIAEoCVIGcmVhc29uEhgKB2V4cGlyZXMYByABKAlSB2V4cGlyZXMSFgoGYWN0'
    'aXZlGAggASgIUgZhY3RpdmU=');

@$core.Deprecated('Use listExceptionsRequestDescriptor instead')
const ListExceptionsRequest$json = {
  '1': 'ListExceptionsRequest',
  '2': [
    {'1': 'include_inactive', '3': 1, '4': 1, '5': 8, '10': 'includeInactive'},
  ],
};

/// Descriptor for `ListExceptionsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listExceptionsRequestDescriptor = $convert.base64Decode(
    'ChVMaXN0RXhjZXB0aW9uc1JlcXVlc3QSKQoQaW5jbHVkZV9pbmFjdGl2ZRgBIAEoCFIPaW5jbH'
    'VkZUluYWN0aXZl');

@$core.Deprecated('Use listExceptionsResponseDescriptor instead')
const ListExceptionsResponse$json = {
  '1': 'ListExceptionsResponse',
  '2': [
    {
      '1': 'rules',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.ExceptionRule',
      '10': 'rules'
    },
  ],
};

/// Descriptor for `ListExceptionsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listExceptionsResponseDescriptor =
    $convert.base64Decode(
        'ChZMaXN0RXhjZXB0aW9uc1Jlc3BvbnNlEisKBXJ1bGVzGAEgAygLMhUuaXRjbWRyLkV4Y2VwdG'
        'lvblJ1bGVSBXJ1bGVz');

@$core.Deprecated('Use saveExceptionRequestDescriptor instead')
const SaveExceptionRequest$json = {
  '1': 'SaveExceptionRequest',
  '2': [
    {
      '1': 'rule',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.ExceptionRule',
      '10': 'rule'
    },
  ],
};

/// Descriptor for `SaveExceptionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List saveExceptionRequestDescriptor = $convert.base64Decode(
    'ChRTYXZlRXhjZXB0aW9uUmVxdWVzdBIpCgRydWxlGAEgASgLMhUuaXRjbWRyLkV4Y2VwdGlvbl'
    'J1bGVSBHJ1bGU=');

@$core.Deprecated('Use saveExceptionResponseDescriptor instead')
const SaveExceptionResponse$json = {
  '1': 'SaveExceptionResponse',
  '2': [
    {
      '1': 'rule',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.ExceptionRule',
      '10': 'rule'
    },
  ],
};

/// Descriptor for `SaveExceptionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List saveExceptionResponseDescriptor = $convert.base64Decode(
    'ChVTYXZlRXhjZXB0aW9uUmVzcG9uc2USKQoEcnVsZRgBIAEoCzIVLml0Y21kci5FeGNlcHRpb2'
    '5SdWxlUgRydWxl');

@$core.Deprecated('Use expireExceptionRequestDescriptor instead')
const ExpireExceptionRequest$json = {
  '1': 'ExpireExceptionRequest',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
  ],
};

/// Descriptor for `ExpireExceptionRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List expireExceptionRequestDescriptor = $convert
    .base64Decode('ChZFeHBpcmVFeGNlcHRpb25SZXF1ZXN0Eg4KAmlkGAEgASgJUgJpZA==');

@$core.Deprecated('Use expireExceptionResponseDescriptor instead')
const ExpireExceptionResponse$json = {
  '1': 'ExpireExceptionResponse',
};

/// Descriptor for `ExpireExceptionResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List expireExceptionResponseDescriptor =
    $convert.base64Decode('ChdFeHBpcmVFeGNlcHRpb25SZXNwb25zZQ==');

@$core.Deprecated('Use searchLogsRequestDescriptor instead')
const SearchLogsRequest$json = {
  '1': 'SearchLogsRequest',
  '2': [
    {'1': 'query', '3': 1, '4': 1, '5': 9, '10': 'query'},
    {'1': 'machine', '3': 2, '4': 1, '5': 9, '10': 'machine'},
    {'1': 'start_ts', '3': 3, '4': 1, '5': 3, '10': 'startTs'},
    {'1': 'end_ts', '3': 4, '4': 1, '5': 3, '10': 'endTs'},
    {'1': 'limit', '3': 5, '4': 1, '5': 5, '10': 'limit'},
    {'1': 'category', '3': 6, '4': 1, '5': 9, '10': 'category'},
    {'1': 'status', '3': 7, '4': 1, '5': 9, '10': 'status'},
    {'1': 'offset', '3': 8, '4': 1, '5': 5, '10': 'offset'},
  ],
};

/// Descriptor for `SearchLogsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List searchLogsRequestDescriptor = $convert.base64Decode(
    'ChFTZWFyY2hMb2dzUmVxdWVzdBIUCgVxdWVyeRgBIAEoCVIFcXVlcnkSGAoHbWFjaGluZRgCIA'
    'EoCVIHbWFjaGluZRIZCghzdGFydF90cxgDIAEoA1IHc3RhcnRUcxIVCgZlbmRfdHMYBCABKANS'
    'BWVuZFRzEhQKBWxpbWl0GAUgASgFUgVsaW1pdBIaCghjYXRlZ29yeRgGIAEoCVIIY2F0ZWdvcn'
    'kSFgoGc3RhdHVzGAcgASgJUgZzdGF0dXMSFgoGb2Zmc2V0GAggASgFUgZvZmZzZXQ=');

@$core.Deprecated('Use searchLogsResponseDescriptor instead')
const SearchLogsResponse$json = {
  '1': 'SearchLogsResponse',
  '2': [
    {'1': 'lines', '3': 1, '4': 3, '5': 9, '10': 'lines'},
    {'1': 'total', '3': 2, '4': 1, '5': 5, '10': 'total'},
    {'1': 'next_offset', '3': 3, '4': 1, '5': 5, '10': 'nextOffset'},
  ],
};

/// Descriptor for `SearchLogsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List searchLogsResponseDescriptor = $convert.base64Decode(
    'ChJTZWFyY2hMb2dzUmVzcG9uc2USFAoFbGluZXMYASADKAlSBWxpbmVzEhQKBXRvdGFsGAIgAS'
    'gFUgV0b3RhbBIfCgtuZXh0X29mZnNldBgDIAEoBVIKbmV4dE9mZnNldA==');

@$core.Deprecated('Use auditEntryDescriptor instead')
const AuditEntry$json = {
  '1': 'AuditEntry',
  '2': [
    {'1': 'ts', '3': 1, '4': 1, '5': 3, '10': 'ts'},
    {'1': 'actor', '3': 2, '4': 1, '5': 9, '10': 'actor'},
    {'1': 'action', '3': 3, '4': 1, '5': 9, '10': 'action'},
    {'1': 'detail', '3': 4, '4': 1, '5': 9, '10': 'detail'},
  ],
};

/// Descriptor for `AuditEntry`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List auditEntryDescriptor = $convert.base64Decode(
    'CgpBdWRpdEVudHJ5Eg4KAnRzGAEgASgDUgJ0cxIUCgVhY3RvchgCIAEoCVIFYWN0b3ISFgoGYW'
    'N0aW9uGAMgASgJUgZhY3Rpb24SFgoGZGV0YWlsGAQgASgJUgZkZXRhaWw=');

@$core.Deprecated('Use listAuditRequestDescriptor instead')
const ListAuditRequest$json = {
  '1': 'ListAuditRequest',
  '2': [
    {'1': 'limit', '3': 1, '4': 1, '5': 5, '10': 'limit'},
  ],
};

/// Descriptor for `ListAuditRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listAuditRequestDescriptor = $convert
    .base64Decode('ChBMaXN0QXVkaXRSZXF1ZXN0EhQKBWxpbWl0GAEgASgFUgVsaW1pdA==');

@$core.Deprecated('Use listAuditResponseDescriptor instead')
const ListAuditResponse$json = {
  '1': 'ListAuditResponse',
  '2': [
    {
      '1': 'entries',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.AuditEntry',
      '10': 'entries'
    },
  ],
};

/// Descriptor for `ListAuditResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listAuditResponseDescriptor = $convert.base64Decode(
    'ChFMaXN0QXVkaXRSZXNwb25zZRIsCgdlbnRyaWVzGAEgAygLMhIuaXRjbWRyLkF1ZGl0RW50cn'
    'lSB2VudHJpZXM=');

@$core.Deprecated('Use watchUpdatesRequestDescriptor instead')
const WatchUpdatesRequest$json = {
  '1': 'WatchUpdatesRequest',
};

/// Descriptor for `WatchUpdatesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List watchUpdatesRequestDescriptor =
    $convert.base64Decode('ChNXYXRjaFVwZGF0ZXNSZXF1ZXN0');

@$core.Deprecated('Use updateDescriptor instead')
const Update$json = {
  '1': 'Update',
  '2': [
    {'1': 'kind', '3': 1, '4': 1, '5': 9, '10': 'kind'},
    {'1': 'entity', '3': 2, '4': 1, '5': 9, '10': 'entity'},
    {'1': 'id', '3': 3, '4': 1, '5': 9, '10': 'id'},
    {'1': 'ts', '3': 4, '4': 1, '5': 3, '10': 'ts'},
  ],
};

/// Descriptor for `Update`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateDescriptor = $convert.base64Decode(
    'CgZVcGRhdGUSEgoEa2luZBgBIAEoCVIEa2luZBIWCgZlbnRpdHkYAiABKAlSBmVudGl0eRIOCg'
    'JpZBgDIAEoCVICaWQSDgoCdHMYBCABKANSAnRz');

@$core.Deprecated('Use backupInfoDescriptor instead')
const BackupInfo$json = {
  '1': 'BackupInfo',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {'1': 'size', '3': 2, '4': 1, '5': 3, '10': 'size'},
    {'1': 'created_ts', '3': 3, '4': 1, '5': 3, '10': 'createdTs'},
    {'1': 'kind', '3': 4, '4': 1, '5': 9, '10': 'kind'},
    {'1': 'itcmdr_version', '3': 5, '4': 1, '5': 9, '10': 'itcmdrVersion'},
    {'1': 'schema_version', '3': 6, '4': 1, '5': 3, '10': 'schemaVersion'},
  ],
};

/// Descriptor for `BackupInfo`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List backupInfoDescriptor = $convert.base64Decode(
    'CgpCYWNrdXBJbmZvEhIKBG5hbWUYASABKAlSBG5hbWUSEgoEc2l6ZRgCIAEoA1IEc2l6ZRIdCg'
    'pjcmVhdGVkX3RzGAMgASgDUgljcmVhdGVkVHMSEgoEa2luZBgEIAEoCVIEa2luZBIlCg5pdGNt'
    'ZHJfdmVyc2lvbhgFIAEoCVINaXRjbWRyVmVyc2lvbhIlCg5zY2hlbWFfdmVyc2lvbhgGIAEoA1'
    'INc2NoZW1hVmVyc2lvbg==');

@$core.Deprecated('Use createBackupRequestDescriptor instead')
const CreateBackupRequest$json = {
  '1': 'CreateBackupRequest',
};

/// Descriptor for `CreateBackupRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List createBackupRequestDescriptor =
    $convert.base64Decode('ChNDcmVhdGVCYWNrdXBSZXF1ZXN0');

@$core.Deprecated('Use createBackupResponseDescriptor instead')
const CreateBackupResponse$json = {
  '1': 'CreateBackupResponse',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupInfo',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `CreateBackupResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List createBackupResponseDescriptor = $convert.base64Decode(
    'ChRDcmVhdGVCYWNrdXBSZXNwb25zZRIqCgZiYWNrdXAYASABKAsyEi5pdGNtZHIuQmFja3VwSW'
    '5mb1IGYmFja3Vw');

@$core.Deprecated('Use listBackupsRequestDescriptor instead')
const ListBackupsRequest$json = {
  '1': 'ListBackupsRequest',
};

/// Descriptor for `ListBackupsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listBackupsRequestDescriptor =
    $convert.base64Decode('ChJMaXN0QmFja3Vwc1JlcXVlc3Q=');

@$core.Deprecated('Use listBackupsResponseDescriptor instead')
const ListBackupsResponse$json = {
  '1': 'ListBackupsResponse',
  '2': [
    {
      '1': 'backups',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.BackupInfo',
      '10': 'backups'
    },
  ],
};

/// Descriptor for `ListBackupsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listBackupsResponseDescriptor = $convert.base64Decode(
    'ChNMaXN0QmFja3Vwc1Jlc3BvbnNlEiwKB2JhY2t1cHMYASADKAsyEi5pdGNtZHIuQmFja3VwSW'
    '5mb1IHYmFja3Vwcw==');

@$core.Deprecated('Use verifyBackupRequestDescriptor instead')
const VerifyBackupRequest$json = {
  '1': 'VerifyBackupRequest',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
  ],
};

/// Descriptor for `VerifyBackupRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List verifyBackupRequestDescriptor = $convert
    .base64Decode('ChNWZXJpZnlCYWNrdXBSZXF1ZXN0EhIKBG5hbWUYASABKAlSBG5hbWU=');

@$core.Deprecated('Use verifyBackupResponseDescriptor instead')
const VerifyBackupResponse$json = {
  '1': 'VerifyBackupResponse',
  '2': [
    {'1': 'ok', '3': 1, '4': 1, '5': 8, '10': 'ok'},
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
  ],
};

/// Descriptor for `VerifyBackupResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List verifyBackupResponseDescriptor = $convert.base64Decode(
    'ChRWZXJpZnlCYWNrdXBSZXNwb25zZRIOCgJvaxgBIAEoCFICb2sSGAoHbWVzc2FnZRgCIAEoCV'
    'IHbWVzc2FnZQ==');

@$core.Deprecated('Use backupSettingsDescriptor instead')
const BackupSettings$json = {
  '1': 'BackupSettings',
  '2': [
    {'1': 'dir', '3': 1, '4': 1, '5': 9, '10': 'dir'},
    {'1': 'nightly', '3': 2, '4': 1, '5': 8, '10': 'nightly'},
    {'1': 'hour', '3': 3, '4': 1, '5': 13, '10': 'hour'},
    {'1': 'keep_daily', '3': 4, '4': 1, '5': 13, '10': 'keepDaily'},
    {'1': 'keep_weekly', '3': 5, '4': 1, '5': 13, '10': 'keepWeekly'},
    {'1': 'keep_blocks', '3': 6, '4': 1, '5': 13, '10': 'keepBlocks'},
  ],
};

/// Descriptor for `BackupSettings`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List backupSettingsDescriptor = $convert.base64Decode(
    'Cg5CYWNrdXBTZXR0aW5ncxIQCgNkaXIYASABKAlSA2RpchIYCgduaWdodGx5GAIgASgIUgduaW'
    'dodGx5EhIKBGhvdXIYAyABKA1SBGhvdXISHQoKa2VlcF9kYWlseRgEIAEoDVIJa2VlcERhaWx5'
    'Eh8KC2tlZXBfd2Vla2x5GAUgASgNUgprZWVwV2Vla2x5Eh8KC2tlZXBfYmxvY2tzGAYgASgNUg'
    'prZWVwQmxvY2tz');

@$core.Deprecated('Use getSettingsRequestDescriptor instead')
const GetSettingsRequest$json = {
  '1': 'GetSettingsRequest',
};

/// Descriptor for `GetSettingsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSettingsRequestDescriptor =
    $convert.base64Decode('ChJHZXRTZXR0aW5nc1JlcXVlc3Q=');

@$core.Deprecated('Use getSettingsResponseDescriptor instead')
const GetSettingsResponse$json = {
  '1': 'GetSettingsResponse',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupSettings',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `GetSettingsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getSettingsResponseDescriptor = $convert.base64Decode(
    'ChNHZXRTZXR0aW5nc1Jlc3BvbnNlEi4KBmJhY2t1cBgBIAEoCzIWLml0Y21kci5CYWNrdXBTZX'
    'R0aW5nc1IGYmFja3Vw');

@$core.Deprecated('Use updateSettingsRequestDescriptor instead')
const UpdateSettingsRequest$json = {
  '1': 'UpdateSettingsRequest',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupSettings',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `UpdateSettingsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingsRequestDescriptor = $convert.base64Decode(
    'ChVVcGRhdGVTZXR0aW5nc1JlcXVlc3QSLgoGYmFja3VwGAEgASgLMhYuaXRjbWRyLkJhY2t1cF'
    'NldHRpbmdzUgZiYWNrdXA=');

@$core.Deprecated('Use updateSettingsResponseDescriptor instead')
const UpdateSettingsResponse$json = {
  '1': 'UpdateSettingsResponse',
  '2': [
    {
      '1': 'backup',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.BackupSettings',
      '10': 'backup'
    },
  ],
};

/// Descriptor for `UpdateSettingsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingsResponseDescriptor =
    $convert.base64Decode(
        'ChZVcGRhdGVTZXR0aW5nc1Jlc3BvbnNlEi4KBmJhY2t1cBgBIAEoCzIWLml0Y21kci5CYWNrdX'
        'BTZXR0aW5nc1IGYmFja3Vw');

@$core.Deprecated('Use settingItemDescriptor instead')
const SettingItem$json = {
  '1': 'SettingItem',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'scope', '3': 2, '4': 1, '5': 9, '10': 'scope'},
    {'1': 'type', '3': 3, '4': 1, '5': 9, '10': 'type'},
    {'1': 'value', '3': 4, '4': 1, '5': 9, '10': 'value'},
    {'1': 'default_value', '3': 5, '4': 1, '5': 9, '10': 'defaultValue'},
    {'1': 'source', '3': 6, '4': 1, '5': 9, '10': 'source'},
    {'1': 'description', '3': 7, '4': 1, '5': 9, '10': 'description'},
    {'1': 'min', '3': 8, '4': 1, '5': 3, '10': 'min'},
    {'1': 'max', '3': 9, '4': 1, '5': 3, '10': 'max'},
    {'1': 'read_only', '3': 10, '4': 1, '5': 8, '10': 'readOnly'},
    {'1': 'applies', '3': 11, '4': 1, '5': 9, '10': 'applies'},
  ],
};

/// Descriptor for `SettingItem`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List settingItemDescriptor = $convert.base64Decode(
    'CgtTZXR0aW5nSXRlbRIQCgNrZXkYASABKAlSA2tleRIUCgVzY29wZRgCIAEoCVIFc2NvcGUSEg'
    'oEdHlwZRgDIAEoCVIEdHlwZRIUCgV2YWx1ZRgEIAEoCVIFdmFsdWUSIwoNZGVmYXVsdF92YWx1'
    'ZRgFIAEoCVIMZGVmYXVsdFZhbHVlEhYKBnNvdXJjZRgGIAEoCVIGc291cmNlEiAKC2Rlc2NyaX'
    'B0aW9uGAcgASgJUgtkZXNjcmlwdGlvbhIQCgNtaW4YCCABKANSA21pbhIQCgNtYXgYCSABKANS'
    'A21heBIbCglyZWFkX29ubHkYCiABKAhSCHJlYWRPbmx5EhgKB2FwcGxpZXMYCyABKAlSB2FwcG'
    'xpZXM=');

@$core.Deprecated('Use listSettingsRequestDescriptor instead')
const ListSettingsRequest$json = {
  '1': 'ListSettingsRequest',
  '2': [
    {'1': 'machine', '3': 1, '4': 1, '5': 9, '10': 'machine'},
  ],
};

/// Descriptor for `ListSettingsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listSettingsRequestDescriptor =
    $convert.base64Decode(
        'ChNMaXN0U2V0dGluZ3NSZXF1ZXN0EhgKB21hY2hpbmUYASABKAlSB21hY2hpbmU=');

@$core.Deprecated('Use listSettingsResponseDescriptor instead')
const ListSettingsResponse$json = {
  '1': 'ListSettingsResponse',
  '2': [
    {
      '1': 'settings',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.SettingItem',
      '10': 'settings'
    },
    {
      '1': 'machines_with_overrides',
      '3': 2,
      '4': 3,
      '5': 9,
      '10': 'machinesWithOverrides'
    },
  ],
};

/// Descriptor for `ListSettingsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listSettingsResponseDescriptor = $convert.base64Decode(
    'ChRMaXN0U2V0dGluZ3NSZXNwb25zZRIvCghzZXR0aW5ncxgBIAMoCzITLml0Y21kci5TZXR0aW'
    '5nSXRlbVIIc2V0dGluZ3MSNgoXbWFjaGluZXNfd2l0aF9vdmVycmlkZXMYAiADKAlSFW1hY2hp'
    'bmVzV2l0aE92ZXJyaWRlcw==');

@$core.Deprecated('Use updateSettingRequestDescriptor instead')
const UpdateSettingRequest$json = {
  '1': 'UpdateSettingRequest',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'value', '3': 2, '4': 1, '5': 9, '10': 'value'},
    {'1': 'machine', '3': 3, '4': 1, '5': 9, '10': 'machine'},
    {'1': 'unset', '3': 4, '4': 1, '5': 8, '10': 'unset'},
  ],
};

/// Descriptor for `UpdateSettingRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingRequestDescriptor = $convert.base64Decode(
    'ChRVcGRhdGVTZXR0aW5nUmVxdWVzdBIQCgNrZXkYASABKAlSA2tleRIUCgV2YWx1ZRgCIAEoCV'
    'IFdmFsdWUSGAoHbWFjaGluZRgDIAEoCVIHbWFjaGluZRIUCgV1bnNldBgEIAEoCFIFdW5zZXQ=');

@$core.Deprecated('Use updateSettingResponseDescriptor instead')
const UpdateSettingResponse$json = {
  '1': 'UpdateSettingResponse',
  '2': [
    {
      '1': 'setting',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.SettingItem',
      '10': 'setting'
    },
  ],
};

/// Descriptor for `UpdateSettingResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List updateSettingResponseDescriptor = $convert.base64Decode(
    'ChVVcGRhdGVTZXR0aW5nUmVzcG9uc2USLQoHc2V0dGluZxgBIAEoCzITLml0Y21kci5TZXR0aW'
    '5nSXRlbVIHc2V0dGluZw==');

@$core.Deprecated('Use runPluginRequestDescriptor instead')
const RunPluginRequest$json = {
  '1': 'RunPluginRequest',
  '2': [
    {'1': 'plugin', '3': 1, '4': 1, '5': 9, '10': 'plugin'},
    {'1': 'report_only', '3': 2, '4': 1, '5': 8, '10': 'reportOnly'},
    {
      '1': 'args',
      '3': 3,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.RunPluginRequest.ArgsEntry',
      '10': 'args'
    },
    {'1': 'machine', '3': 4, '4': 1, '5': 9, '10': 'machine'},
  ],
  '3': [RunPluginRequest_ArgsEntry$json],
};

@$core.Deprecated('Use runPluginRequestDescriptor instead')
const RunPluginRequest_ArgsEntry$json = {
  '1': 'ArgsEntry',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'value', '3': 2, '4': 1, '5': 9, '10': 'value'},
  ],
  '7': {'7': true},
};

/// Descriptor for `RunPluginRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List runPluginRequestDescriptor = $convert.base64Decode(
    'ChBSdW5QbHVnaW5SZXF1ZXN0EhYKBnBsdWdpbhgBIAEoCVIGcGx1Z2luEh8KC3JlcG9ydF9vbm'
    'x5GAIgASgIUgpyZXBvcnRPbmx5EjYKBGFyZ3MYAyADKAsyIi5pdGNtZHIuUnVuUGx1Z2luUmVx'
    'dWVzdC5BcmdzRW50cnlSBGFyZ3MSGAoHbWFjaGluZRgEIAEoCVIHbWFjaGluZRo3CglBcmdzRW'
    '50cnkSEAoDa2V5GAEgASgJUgNrZXkSFAoFdmFsdWUYAiABKAlSBXZhbHVlOgI4AQ==');

@$core.Deprecated('Use pluginStageDescriptor instead')
const PluginStage$json = {
  '1': 'PluginStage',
  '2': [
    {'1': 'stage', '3': 1, '4': 1, '5': 9, '10': 'stage'},
    {'1': 'ok', '3': 2, '4': 1, '5': 8, '10': 'ok'},
    {'1': 'message', '3': 3, '4': 1, '5': 9, '10': 'message'},
  ],
};

/// Descriptor for `PluginStage`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List pluginStageDescriptor = $convert.base64Decode(
    'CgtQbHVnaW5TdGFnZRIUCgVzdGFnZRgBIAEoCVIFc3RhZ2USDgoCb2sYAiABKAhSAm9rEhgKB2'
    '1lc3NhZ2UYAyABKAlSB21lc3NhZ2U=');

@$core.Deprecated('Use pluginCheckResultDescriptor instead')
const PluginCheckResult$json = {
  '1': 'PluginCheckResult',
  '2': [
    {'1': 'check_id', '3': 1, '4': 1, '5': 9, '10': 'checkId'},
    {'1': 'status', '3': 2, '4': 1, '5': 9, '10': 'status'},
    {'1': 'message', '3': 3, '4': 1, '5': 9, '10': 'message'},
  ],
};

/// Descriptor for `PluginCheckResult`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List pluginCheckResultDescriptor = $convert.base64Decode(
    'ChFQbHVnaW5DaGVja1Jlc3VsdBIZCghjaGVja19pZBgBIAEoCVIHY2hlY2tJZBIWCgZzdGF0dX'
    'MYAiABKAlSBnN0YXR1cxIYCgdtZXNzYWdlGAMgASgJUgdtZXNzYWdl');

@$core.Deprecated('Use runPluginResponseDescriptor instead')
const RunPluginResponse$json = {
  '1': 'RunPluginResponse',
  '2': [
    {'1': 'run_id', '3': 1, '4': 1, '5': 9, '10': 'runId'},
    {'1': 'status', '3': 2, '4': 1, '5': 9, '10': 'status'},
    {'1': 'message', '3': 3, '4': 1, '5': 9, '10': 'message'},
    {
      '1': 'stages',
      '3': 4,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.PluginStage',
      '10': 'stages'
    },
    {
      '1': 'results',
      '3': 5,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.PluginCheckResult',
      '10': 'results'
    },
  ],
};

/// Descriptor for `RunPluginResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List runPluginResponseDescriptor = $convert.base64Decode(
    'ChFSdW5QbHVnaW5SZXNwb25zZRIVCgZydW5faWQYASABKAlSBXJ1bklkEhYKBnN0YXR1cxgCIA'
    'EoCVIGc3RhdHVzEhgKB21lc3NhZ2UYAyABKAlSB21lc3NhZ2USKwoGc3RhZ2VzGAQgAygLMhMu'
    'aXRjbWRyLlBsdWdpblN0YWdlUgZzdGFnZXMSMwoHcmVzdWx0cxgFIAMoCzIZLml0Y21kci5QbH'
    'VnaW5DaGVja1Jlc3VsdFIHcmVzdWx0cw==');

@$core.Deprecated('Use listPluginRunsRequestDescriptor instead')
const ListPluginRunsRequest$json = {
  '1': 'ListPluginRunsRequest',
  '2': [
    {'1': 'plugin', '3': 1, '4': 1, '5': 9, '10': 'plugin'},
    {'1': 'limit', '3': 2, '4': 1, '5': 5, '10': 'limit'},
  ],
};

/// Descriptor for `ListPluginRunsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listPluginRunsRequestDescriptor = $convert.base64Decode(
    'ChVMaXN0UGx1Z2luUnVuc1JlcXVlc3QSFgoGcGx1Z2luGAEgASgJUgZwbHVnaW4SFAoFbGltaX'
    'QYAiABKAVSBWxpbWl0');

@$core.Deprecated('Use pluginRunDescriptor instead')
const PluginRun$json = {
  '1': 'PluginRun',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {'1': 'plugin', '3': 2, '4': 1, '5': 9, '10': 'plugin'},
    {'1': 'version', '3': 3, '4': 1, '5': 9, '10': 'version'},
    {'1': 'machine', '3': 4, '4': 1, '5': 9, '10': 'machine'},
    {'1': 'started', '3': 5, '4': 1, '5': 3, '10': 'started'},
    {'1': 'finished', '3': 6, '4': 1, '5': 3, '10': 'finished'},
    {'1': 'status', '3': 7, '4': 1, '5': 9, '10': 'status'},
    {'1': 'message', '3': 8, '4': 1, '5': 9, '10': 'message'},
    {'1': 'request_id', '3': 9, '4': 1, '5': 9, '10': 'requestId'},
  ],
};

/// Descriptor for `PluginRun`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List pluginRunDescriptor = $convert.base64Decode(
    'CglQbHVnaW5SdW4SDgoCaWQYASABKAlSAmlkEhYKBnBsdWdpbhgCIAEoCVIGcGx1Z2luEhgKB3'
    'ZlcnNpb24YAyABKAlSB3ZlcnNpb24SGAoHbWFjaGluZRgEIAEoCVIHbWFjaGluZRIYCgdzdGFy'
    'dGVkGAUgASgDUgdzdGFydGVkEhoKCGZpbmlzaGVkGAYgASgDUghmaW5pc2hlZBIWCgZzdGF0dX'
    'MYByABKAlSBnN0YXR1cxIYCgdtZXNzYWdlGAggASgJUgdtZXNzYWdlEh0KCnJlcXVlc3RfaWQY'
    'CSABKAlSCXJlcXVlc3RJZA==');

@$core.Deprecated('Use listPluginRunsResponseDescriptor instead')
const ListPluginRunsResponse$json = {
  '1': 'ListPluginRunsResponse',
  '2': [
    {
      '1': 'runs',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.PluginRun',
      '10': 'runs'
    },
  ],
};

/// Descriptor for `ListPluginRunsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listPluginRunsResponseDescriptor =
    $convert.base64Decode(
        'ChZMaXN0UGx1Z2luUnVuc1Jlc3BvbnNlEiUKBHJ1bnMYASADKAsyES5pdGNtZHIuUGx1Z2luUn'
        'VuUgRydW5z');

@$core.Deprecated('Use getPluginMetricsRequestDescriptor instead')
const GetPluginMetricsRequest$json = {
  '1': 'GetPluginMetricsRequest',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {
      '1': 'labels',
      '3': 2,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.GetPluginMetricsRequest.LabelsEntry',
      '10': 'labels'
    },
    {'1': 'window_days', '3': 3, '4': 1, '5': 5, '10': 'windowDays'},
    {'1': 'daily', '3': 4, '4': 1, '5': 8, '10': 'daily'},
  ],
  '3': [GetPluginMetricsRequest_LabelsEntry$json],
};

@$core.Deprecated('Use getPluginMetricsRequestDescriptor instead')
const GetPluginMetricsRequest_LabelsEntry$json = {
  '1': 'LabelsEntry',
  '2': [
    {'1': 'key', '3': 1, '4': 1, '5': 9, '10': 'key'},
    {'1': 'value', '3': 2, '4': 1, '5': 9, '10': 'value'},
  ],
  '7': {'7': true},
};

/// Descriptor for `GetPluginMetricsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getPluginMetricsRequestDescriptor = $convert.base64Decode(
    'ChdHZXRQbHVnaW5NZXRyaWNzUmVxdWVzdBISCgRuYW1lGAEgASgJUgRuYW1lEkMKBmxhYmVscx'
    'gCIAMoCzIrLml0Y21kci5HZXRQbHVnaW5NZXRyaWNzUmVxdWVzdC5MYWJlbHNFbnRyeVIGbGFi'
    'ZWxzEh8KC3dpbmRvd19kYXlzGAMgASgFUgp3aW5kb3dEYXlzEhQKBWRhaWx5GAQgASgIUgVkYW'
    'lseRo5CgtMYWJlbHNFbnRyeRIQCgNrZXkYASABKAlSA2tleRIUCgV2YWx1ZRgCIAEoCVIFdmFs'
    'dWU6AjgB');

@$core.Deprecated('Use pluginMetricPointDescriptor instead')
const PluginMetricPoint$json = {
  '1': 'PluginMetricPoint',
  '2': [
    {'1': 'name', '3': 1, '4': 1, '5': 9, '10': 'name'},
    {'1': 'labels', '3': 2, '4': 1, '5': 9, '10': 'labels'},
    {'1': 'ts', '3': 3, '4': 1, '5': 3, '10': 'ts'},
    {'1': 'value', '3': 4, '4': 1, '5': 1, '10': 'value'},
  ],
};

/// Descriptor for `PluginMetricPoint`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List pluginMetricPointDescriptor = $convert.base64Decode(
    'ChFQbHVnaW5NZXRyaWNQb2ludBISCgRuYW1lGAEgASgJUgRuYW1lEhYKBmxhYmVscxgCIAEoCV'
    'IGbGFiZWxzEg4KAnRzGAMgASgDUgJ0cxIUCgV2YWx1ZRgEIAEoAVIFdmFsdWU=');

@$core.Deprecated('Use logRejectsDescriptor instead')
const LogRejects$json = {
  '1': 'LogRejects',
  '2': [
    {'1': 'file', '3': 1, '4': 1, '5': 9, '10': 'file'},
    {'1': 'count', '3': 2, '4': 1, '5': 3, '10': 'count'},
  ],
};

/// Descriptor for `LogRejects`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List logRejectsDescriptor = $convert.base64Decode(
    'CgpMb2dSZWplY3RzEhIKBGZpbGUYASABKAlSBGZpbGUSFAoFY291bnQYAiABKANSBWNvdW50');

@$core.Deprecated('Use getPluginMetricsResponseDescriptor instead')
const GetPluginMetricsResponse$json = {
  '1': 'GetPluginMetricsResponse',
  '2': [
    {
      '1': 'points',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.PluginMetricPoint',
      '10': 'points'
    },
    {
      '1': 'rejects',
      '3': 2,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.LogRejects',
      '10': 'rejects'
    },
  ],
};

/// Descriptor for `GetPluginMetricsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getPluginMetricsResponseDescriptor = $convert.base64Decode(
    'ChhHZXRQbHVnaW5NZXRyaWNzUmVzcG9uc2USMQoGcG9pbnRzGAEgAygLMhkuaXRjbWRyLlBsdW'
    'dpbk1ldHJpY1BvaW50UgZwb2ludHMSLAoHcmVqZWN0cxgCIAMoCzISLml0Y21kci5Mb2dSZWpl'
    'Y3RzUgdyZWplY3Rz');

@$core.Deprecated('Use scheduleDescriptor instead')
const Schedule$json = {
  '1': 'Schedule',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {'1': 'name', '3': 2, '4': 1, '5': 9, '10': 'name'},
    {'1': 'enabled', '3': 3, '4': 1, '5': 8, '10': 'enabled'},
    {'1': 'all_machines', '3': 4, '4': 1, '5': 8, '10': 'allMachines'},
    {'1': 'machines', '3': 5, '4': 3, '5': 9, '10': 'machines'},
    {'1': 'groups', '3': 6, '4': 3, '5': 9, '10': 'groups'},
    {'1': 'categories', '3': 7, '4': 3, '5': 9, '10': 'categories'},
    {'1': 'plugins', '3': 8, '4': 3, '5': 9, '10': 'plugins'},
    {'1': 'repair', '3': 9, '4': 1, '5': 8, '10': 'repair'},
    {'1': 'when', '3': 10, '4': 1, '5': 9, '10': 'when'},
    {'1': 'timezone', '3': 11, '4': 1, '5': 9, '10': 'timezone'},
    {'1': 'guard', '3': 12, '4': 1, '5': 9, '10': 'guard'},
    {'1': 'missed', '3': 13, '4': 1, '5': 9, '10': 'missed'},
    {'1': 'builtin', '3': 14, '4': 1, '5': 8, '10': 'builtin'},
    {'1': 'next_runs', '3': 15, '4': 3, '5': 3, '10': 'nextRuns'},
    {'1': 'last_fire', '3': 16, '4': 1, '5': 3, '10': 'lastFire'},
    {'1': 'builtin_key', '3': 17, '4': 1, '5': 9, '10': 'builtinKey'},
    {'1': 'action', '3': 18, '4': 1, '5': 9, '10': 'action'},
  ],
};

/// Descriptor for `Schedule`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List scheduleDescriptor = $convert.base64Decode(
    'CghTY2hlZHVsZRIOCgJpZBgBIAEoCVICaWQSEgoEbmFtZRgCIAEoCVIEbmFtZRIYCgdlbmFibG'
    'VkGAMgASgIUgdlbmFibGVkEiEKDGFsbF9tYWNoaW5lcxgEIAEoCFILYWxsTWFjaGluZXMSGgoI'
    'bWFjaGluZXMYBSADKAlSCG1hY2hpbmVzEhYKBmdyb3VwcxgGIAMoCVIGZ3JvdXBzEh4KCmNhdG'
    'Vnb3JpZXMYByADKAlSCmNhdGVnb3JpZXMSGAoHcGx1Z2lucxgIIAMoCVIHcGx1Z2lucxIWCgZy'
    'ZXBhaXIYCSABKAhSBnJlcGFpchISCgR3aGVuGAogASgJUgR3aGVuEhoKCHRpbWV6b25lGAsgAS'
    'gJUgh0aW1lem9uZRIUCgVndWFyZBgMIAEoCVIFZ3VhcmQSFgoGbWlzc2VkGA0gASgJUgZtaXNz'
    'ZWQSGAoHYnVpbHRpbhgOIAEoCFIHYnVpbHRpbhIbCgluZXh0X3J1bnMYDyADKANSCG5leHRSdW'
    '5zEhsKCWxhc3RfZmlyZRgQIAEoA1IIbGFzdEZpcmUSHwoLYnVpbHRpbl9rZXkYESABKAlSCmJ1'
    'aWx0aW5LZXkSFgoGYWN0aW9uGBIgASgJUgZhY3Rpb24=');

@$core.Deprecated('Use saveScheduleRequestDescriptor instead')
const SaveScheduleRequest$json = {
  '1': 'SaveScheduleRequest',
  '2': [
    {
      '1': 'schedule',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.Schedule',
      '10': 'schedule'
    },
  ],
};

/// Descriptor for `SaveScheduleRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List saveScheduleRequestDescriptor = $convert.base64Decode(
    'ChNTYXZlU2NoZWR1bGVSZXF1ZXN0EiwKCHNjaGVkdWxlGAEgASgLMhAuaXRjbWRyLlNjaGVkdW'
    'xlUghzY2hlZHVsZQ==');

@$core.Deprecated('Use scheduleResponseDescriptor instead')
const ScheduleResponse$json = {
  '1': 'ScheduleResponse',
  '2': [
    {
      '1': 'schedule',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.Schedule',
      '10': 'schedule'
    },
  ],
};

/// Descriptor for `ScheduleResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List scheduleResponseDescriptor = $convert.base64Decode(
    'ChBTY2hlZHVsZVJlc3BvbnNlEiwKCHNjaGVkdWxlGAEgASgLMhAuaXRjbWRyLlNjaGVkdWxlUg'
    'hzY2hlZHVsZQ==');

@$core.Deprecated('Use scheduleRefDescriptor instead')
const ScheduleRef$json = {
  '1': 'ScheduleRef',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {'1': 'force', '3': 2, '4': 1, '5': 8, '10': 'force'},
    {'1': 'confirm', '3': 3, '4': 1, '5': 9, '10': 'confirm'},
  ],
};

/// Descriptor for `ScheduleRef`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List scheduleRefDescriptor = $convert.base64Decode(
    'CgtTY2hlZHVsZVJlZhIOCgJpZBgBIAEoCVICaWQSFAoFZm9yY2UYAiABKAhSBWZvcmNlEhgKB2'
    'NvbmZpcm0YAyABKAlSB2NvbmZpcm0=');

@$core.Deprecated('Use listSchedulesRequestDescriptor instead')
const ListSchedulesRequest$json = {
  '1': 'ListSchedulesRequest',
};

/// Descriptor for `ListSchedulesRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listSchedulesRequestDescriptor =
    $convert.base64Decode('ChRMaXN0U2NoZWR1bGVzUmVxdWVzdA==');

@$core.Deprecated('Use listSchedulesResponseDescriptor instead')
const ListSchedulesResponse$json = {
  '1': 'ListSchedulesResponse',
  '2': [
    {
      '1': 'schedules',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.Schedule',
      '10': 'schedules'
    },
  ],
};

/// Descriptor for `ListSchedulesResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listSchedulesResponseDescriptor = $convert.base64Decode(
    'ChVMaXN0U2NoZWR1bGVzUmVzcG9uc2USLgoJc2NoZWR1bGVzGAEgAygLMhAuaXRjbWRyLlNjaG'
    'VkdWxlUglzY2hlZHVsZXM=');

@$core.Deprecated('Use runScheduleNowResponseDescriptor instead')
const RunScheduleNowResponse$json = {
  '1': 'RunScheduleNowResponse',
  '2': [
    {'1': 'outcome', '3': 1, '4': 1, '5': 9, '10': 'outcome'},
    {'1': 'request_id', '3': 2, '4': 1, '5': 9, '10': 'requestId'},
    {'1': 'note', '3': 3, '4': 1, '5': 9, '10': 'note'},
  ],
};

/// Descriptor for `RunScheduleNowResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List runScheduleNowResponseDescriptor =
    $convert.base64Decode(
        'ChZSdW5TY2hlZHVsZU5vd1Jlc3BvbnNlEhgKB291dGNvbWUYASABKAlSB291dGNvbWUSHQoKcm'
        'VxdWVzdF9pZBgCIAEoCVIJcmVxdWVzdElkEhIKBG5vdGUYAyABKAlSBG5vdGU=');

@$core.Deprecated('Use listScheduleRunsRequestDescriptor instead')
const ListScheduleRunsRequest$json = {
  '1': 'ListScheduleRunsRequest',
  '2': [
    {'1': 'id', '3': 1, '4': 1, '5': 9, '10': 'id'},
    {'1': 'limit', '3': 2, '4': 1, '5': 5, '10': 'limit'},
  ],
};

/// Descriptor for `ListScheduleRunsRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listScheduleRunsRequestDescriptor =
    $convert.base64Decode(
        'ChdMaXN0U2NoZWR1bGVSdW5zUmVxdWVzdBIOCgJpZBgBIAEoCVICaWQSFAoFbGltaXQYAiABKA'
        'VSBWxpbWl0');

@$core.Deprecated('Use scheduleRunDescriptor instead')
const ScheduleRun$json = {
  '1': 'ScheduleRun',
  '2': [
    {'1': 'schedule_id', '3': 1, '4': 1, '5': 9, '10': 'scheduleId'},
    {'1': 'ts', '3': 2, '4': 1, '5': 3, '10': 'ts'},
    {'1': 'due', '3': 3, '4': 1, '5': 3, '10': 'due'},
    {'1': 'request_id', '3': 4, '4': 1, '5': 9, '10': 'requestId'},
    {'1': 'outcome', '3': 5, '4': 1, '5': 9, '10': 'outcome'},
    {'1': 'note', '3': 6, '4': 1, '5': 9, '10': 'note'},
  ],
};

/// Descriptor for `ScheduleRun`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List scheduleRunDescriptor = $convert.base64Decode(
    'CgtTY2hlZHVsZVJ1bhIfCgtzY2hlZHVsZV9pZBgBIAEoCVIKc2NoZWR1bGVJZBIOCgJ0cxgCIA'
    'EoA1ICdHMSEAoDZHVlGAMgASgDUgNkdWUSHQoKcmVxdWVzdF9pZBgEIAEoCVIJcmVxdWVzdElk'
    'EhgKB291dGNvbWUYBSABKAlSB291dGNvbWUSEgoEbm90ZRgGIAEoCVIEbm90ZQ==');

@$core.Deprecated('Use listScheduleRunsResponseDescriptor instead')
const ListScheduleRunsResponse$json = {
  '1': 'ListScheduleRunsResponse',
  '2': [
    {
      '1': 'runs',
      '3': 1,
      '4': 3,
      '5': 11,
      '6': '.itcmdr.ScheduleRun',
      '10': 'runs'
    },
  ],
};

/// Descriptor for `ListScheduleRunsResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List listScheduleRunsResponseDescriptor =
    $convert.base64Decode(
        'ChhMaXN0U2NoZWR1bGVSdW5zUmVzcG9uc2USJwoEcnVucxgBIAMoCzITLml0Y21kci5TY2hlZH'
        'VsZVJ1blIEcnVucw==');

@$core.Deprecated('Use stageRestoreRequestDescriptor instead')
const StageRestoreRequest$json = {
  '1': 'StageRestoreRequest',
  '2': [
    {'1': 'backup', '3': 1, '4': 1, '5': 9, '10': 'backup'},
    {'1': 'words', '3': 2, '4': 1, '5': 9, '10': 'words'},
  ],
};

/// Descriptor for `StageRestoreRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List stageRestoreRequestDescriptor = $convert.base64Decode(
    'ChNTdGFnZVJlc3RvcmVSZXF1ZXN0EhYKBmJhY2t1cBgBIAEoCVIGYmFja3VwEhQKBXdvcmRzGA'
    'IgASgJUgV3b3Jkcw==');

@$core.Deprecated('Use restoreStatusDescriptor instead')
const RestoreStatus$json = {
  '1': 'RestoreStatus',
  '2': [
    {'1': 'state', '3': 1, '4': 1, '5': 9, '10': 'state'},
    {'1': 'message', '3': 2, '4': 1, '5': 9, '10': 'message'},
    {'1': 'backup', '3': 3, '4': 1, '5': 9, '10': 'backup'},
    {'1': 'actor', '3': 4, '4': 1, '5': 9, '10': 'actor'},
    {'1': 'started', '3': 5, '4': 1, '5': 3, '10': 'started'},
    {'1': 'updated', '3': 6, '4': 1, '5': 3, '10': 'updated'},
    {'1': 'safety_copy', '3': 7, '4': 1, '5': 9, '10': 'safetyCopy'},
  ],
};

/// Descriptor for `RestoreStatus`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List restoreStatusDescriptor = $convert.base64Decode(
    'Cg1SZXN0b3JlU3RhdHVzEhQKBXN0YXRlGAEgASgJUgVzdGF0ZRIYCgdtZXNzYWdlGAIgASgJUg'
    'dtZXNzYWdlEhYKBmJhY2t1cBgDIAEoCVIGYmFja3VwEhQKBWFjdG9yGAQgASgJUgVhY3RvchIY'
    'CgdzdGFydGVkGAUgASgDUgdzdGFydGVkEhgKB3VwZGF0ZWQYBiABKANSB3VwZGF0ZWQSHwoLc2'
    'FmZXR5X2NvcHkYByABKAlSCnNhZmV0eUNvcHk=');

@$core.Deprecated('Use stageRestoreResponseDescriptor instead')
const StageRestoreResponse$json = {
  '1': 'StageRestoreResponse',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.RestoreStatus',
      '10': 'status'
    },
  ],
};

/// Descriptor for `StageRestoreResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List stageRestoreResponseDescriptor = $convert.base64Decode(
    'ChRTdGFnZVJlc3RvcmVSZXNwb25zZRItCgZzdGF0dXMYASABKAsyFS5pdGNtZHIuUmVzdG9yZV'
    'N0YXR1c1IGc3RhdHVz');

@$core.Deprecated('Use getRestoreStatusRequestDescriptor instead')
const GetRestoreStatusRequest$json = {
  '1': 'GetRestoreStatusRequest',
};

/// Descriptor for `GetRestoreStatusRequest`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getRestoreStatusRequestDescriptor =
    $convert.base64Decode('ChdHZXRSZXN0b3JlU3RhdHVzUmVxdWVzdA==');

@$core.Deprecated('Use getRestoreStatusResponseDescriptor instead')
const GetRestoreStatusResponse$json = {
  '1': 'GetRestoreStatusResponse',
  '2': [
    {
      '1': 'status',
      '3': 1,
      '4': 1,
      '5': 11,
      '6': '.itcmdr.RestoreStatus',
      '10': 'status'
    },
  ],
};

/// Descriptor for `GetRestoreStatusResponse`. Decode as a `google.protobuf.DescriptorProto`.
final $typed_data.Uint8List getRestoreStatusResponseDescriptor =
    $convert.base64Decode(
        'ChhHZXRSZXN0b3JlU3RhdHVzUmVzcG9uc2USLQoGc3RhdHVzGAEgASgLMhUuaXRjbWRyLlJlc3'
        'RvcmVTdGF0dXNSBnN0YXR1cw==');
