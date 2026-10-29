// This is a generated file - do not edit.
//
// Generated from itcmdr.proto.

// @dart = 3.3

// ignore_for_file: annotate_overrides, camel_case_types, comment_references
// ignore_for_file: constant_identifier_names
// ignore_for_file: curly_braces_in_flow_control_structures
// ignore_for_file: deprecated_member_use_from_same_package, library_prefixes
// ignore_for_file: non_constant_identifier_names, prefer_relative_imports

import 'dart:async' as $async;
import 'dart:core' as $core;

import 'package:grpc/service_api.dart' as $grpc;
import 'package:protobuf/protobuf.dart' as $pb;

import 'itcmdr.pb.dart' as $0;

export 'itcmdr.pb.dart';

@$pb.GrpcServiceName('itcmdr.Itcmdr')
class ItcmdrClient extends $grpc.Client {
  /// The hostname for this service.
  static const $core.String defaultHost = '';

  /// OAuth scopes needed for the client.
  static const $core.List<$core.String> oauthScopes = [
    '',
  ];

  ItcmdrClient(super.channel, {super.options, super.interceptors});

  $grpc.ResponseFuture<$0.ListMachinesResponse> listMachines(
    $0.ListMachinesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listMachines, request, options: options);
  }

  $grpc.ResponseFuture<$0.TriggerRunsResponse> triggerRuns(
    $0.TriggerRunsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$triggerRuns, request, options: options);
  }

  $grpc.ResponseFuture<$0.GetRequestStatusResponse> getRequestStatus(
    $0.GetRequestStatusRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getRequestStatus, request, options: options);
  }

  // method descriptors

  static final _$listMachines =
      $grpc.ClientMethod<$0.ListMachinesRequest, $0.ListMachinesResponse>(
          '/itcmdr.Itcmdr/ListMachines',
          ($0.ListMachinesRequest value) => value.writeToBuffer(),
          $0.ListMachinesResponse.fromBuffer);
  static final _$triggerRuns =
      $grpc.ClientMethod<$0.TriggerRunsRequest, $0.TriggerRunsResponse>(
          '/itcmdr.Itcmdr/TriggerRuns',
          ($0.TriggerRunsRequest value) => value.writeToBuffer(),
          $0.TriggerRunsResponse.fromBuffer);
  static final _$getRequestStatus = $grpc.ClientMethod<
          $0.GetRequestStatusRequest, $0.GetRequestStatusResponse>(
      '/itcmdr.Itcmdr/GetRequestStatus',
      ($0.GetRequestStatusRequest value) => value.writeToBuffer(),
      $0.GetRequestStatusResponse.fromBuffer);
}

@$pb.GrpcServiceName('itcmdr.Itcmdr')
abstract class ItcmdrServiceBase extends $grpc.Service {
  $core.String get $name => 'itcmdr.Itcmdr';

  ItcmdrServiceBase() {
    $addMethod(
        $grpc.ServiceMethod<$0.ListMachinesRequest, $0.ListMachinesResponse>(
            'ListMachines',
            listMachines_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.ListMachinesRequest.fromBuffer(value),
            ($0.ListMachinesResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.TriggerRunsRequest, $0.TriggerRunsResponse>(
            'TriggerRuns',
            triggerRuns_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.TriggerRunsRequest.fromBuffer(value),
            ($0.TriggerRunsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetRequestStatusRequest,
            $0.GetRequestStatusResponse>(
        'GetRequestStatus',
        getRequestStatus_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.GetRequestStatusRequest.fromBuffer(value),
        ($0.GetRequestStatusResponse value) => value.writeToBuffer()));
  }

  $async.Future<$0.ListMachinesResponse> listMachines_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListMachinesRequest> $request) async {
    return listMachines($call, await $request);
  }

  $async.Future<$0.ListMachinesResponse> listMachines(
      $grpc.ServiceCall call, $0.ListMachinesRequest request);

  $async.Future<$0.TriggerRunsResponse> triggerRuns_Pre($grpc.ServiceCall $call,
      $async.Future<$0.TriggerRunsRequest> $request) async {
    return triggerRuns($call, await $request);
  }

  $async.Future<$0.TriggerRunsResponse> triggerRuns(
      $grpc.ServiceCall call, $0.TriggerRunsRequest request);

  $async.Future<$0.GetRequestStatusResponse> getRequestStatus_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.GetRequestStatusRequest> $request) async {
    return getRequestStatus($call, await $request);
  }

  $async.Future<$0.GetRequestStatusResponse> getRequestStatus(
      $grpc.ServiceCall call, $0.GetRequestStatusRequest request);
}
