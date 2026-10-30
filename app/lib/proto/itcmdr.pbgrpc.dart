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

  $grpc.ResponseFuture<$0.GetMetricsResponse> getMetrics(
    $0.GetMetricsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getMetrics, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListExceptionsResponse> listExceptions(
    $0.ListExceptionsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listExceptions, request, options: options);
  }

  $grpc.ResponseFuture<$0.SaveExceptionResponse> saveException(
    $0.SaveExceptionRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$saveException, request, options: options);
  }

  $grpc.ResponseFuture<$0.ExpireExceptionResponse> expireException(
    $0.ExpireExceptionRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$expireException, request, options: options);
  }

  $grpc.ResponseFuture<$0.SearchLogsResponse> searchLogs(
    $0.SearchLogsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$searchLogs, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListAuditResponse> listAudit(
    $0.ListAuditRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listAudit, request, options: options);
  }

  $grpc.ResponseFuture<$0.CreateBackupResponse> createBackup(
    $0.CreateBackupRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$createBackup, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListBackupsResponse> listBackups(
    $0.ListBackupsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listBackups, request, options: options);
  }

  $grpc.ResponseFuture<$0.VerifyBackupResponse> verifyBackup(
    $0.VerifyBackupRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$verifyBackup, request, options: options);
  }

  $grpc.ResponseFuture<$0.GetSettingsResponse> getSettings(
    $0.GetSettingsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getSettings, request, options: options);
  }

  $grpc.ResponseFuture<$0.UpdateSettingsResponse> updateSettings(
    $0.UpdateSettingsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$updateSettings, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListSettingsResponse> listSettings(
    $0.ListSettingsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listSettings, request, options: options);
  }

  $grpc.ResponseFuture<$0.UpdateSettingResponse> updateSetting(
    $0.UpdateSettingRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$updateSetting, request, options: options);
  }

  $grpc.ResponseStream<$0.Update> watchUpdates(
    $0.WatchUpdatesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createStreamingCall(
        _$watchUpdates, $async.Stream.fromIterable([request]),
        options: options);
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
  static final _$getMetrics =
      $grpc.ClientMethod<$0.GetMetricsRequest, $0.GetMetricsResponse>(
          '/itcmdr.Itcmdr/GetMetrics',
          ($0.GetMetricsRequest value) => value.writeToBuffer(),
          $0.GetMetricsResponse.fromBuffer);
  static final _$listExceptions =
      $grpc.ClientMethod<$0.ListExceptionsRequest, $0.ListExceptionsResponse>(
          '/itcmdr.Itcmdr/ListExceptions',
          ($0.ListExceptionsRequest value) => value.writeToBuffer(),
          $0.ListExceptionsResponse.fromBuffer);
  static final _$saveException =
      $grpc.ClientMethod<$0.SaveExceptionRequest, $0.SaveExceptionResponse>(
          '/itcmdr.Itcmdr/SaveException',
          ($0.SaveExceptionRequest value) => value.writeToBuffer(),
          $0.SaveExceptionResponse.fromBuffer);
  static final _$expireException =
      $grpc.ClientMethod<$0.ExpireExceptionRequest, $0.ExpireExceptionResponse>(
          '/itcmdr.Itcmdr/ExpireException',
          ($0.ExpireExceptionRequest value) => value.writeToBuffer(),
          $0.ExpireExceptionResponse.fromBuffer);
  static final _$searchLogs =
      $grpc.ClientMethod<$0.SearchLogsRequest, $0.SearchLogsResponse>(
          '/itcmdr.Itcmdr/SearchLogs',
          ($0.SearchLogsRequest value) => value.writeToBuffer(),
          $0.SearchLogsResponse.fromBuffer);
  static final _$listAudit =
      $grpc.ClientMethod<$0.ListAuditRequest, $0.ListAuditResponse>(
          '/itcmdr.Itcmdr/ListAudit',
          ($0.ListAuditRequest value) => value.writeToBuffer(),
          $0.ListAuditResponse.fromBuffer);
  static final _$createBackup =
      $grpc.ClientMethod<$0.CreateBackupRequest, $0.CreateBackupResponse>(
          '/itcmdr.Itcmdr/CreateBackup',
          ($0.CreateBackupRequest value) => value.writeToBuffer(),
          $0.CreateBackupResponse.fromBuffer);
  static final _$listBackups =
      $grpc.ClientMethod<$0.ListBackupsRequest, $0.ListBackupsResponse>(
          '/itcmdr.Itcmdr/ListBackups',
          ($0.ListBackupsRequest value) => value.writeToBuffer(),
          $0.ListBackupsResponse.fromBuffer);
  static final _$verifyBackup =
      $grpc.ClientMethod<$0.VerifyBackupRequest, $0.VerifyBackupResponse>(
          '/itcmdr.Itcmdr/VerifyBackup',
          ($0.VerifyBackupRequest value) => value.writeToBuffer(),
          $0.VerifyBackupResponse.fromBuffer);
  static final _$getSettings =
      $grpc.ClientMethod<$0.GetSettingsRequest, $0.GetSettingsResponse>(
          '/itcmdr.Itcmdr/GetSettings',
          ($0.GetSettingsRequest value) => value.writeToBuffer(),
          $0.GetSettingsResponse.fromBuffer);
  static final _$updateSettings =
      $grpc.ClientMethod<$0.UpdateSettingsRequest, $0.UpdateSettingsResponse>(
          '/itcmdr.Itcmdr/UpdateSettings',
          ($0.UpdateSettingsRequest value) => value.writeToBuffer(),
          $0.UpdateSettingsResponse.fromBuffer);
  static final _$listSettings =
      $grpc.ClientMethod<$0.ListSettingsRequest, $0.ListSettingsResponse>(
          '/itcmdr.Itcmdr/ListSettings',
          ($0.ListSettingsRequest value) => value.writeToBuffer(),
          $0.ListSettingsResponse.fromBuffer);
  static final _$updateSetting =
      $grpc.ClientMethod<$0.UpdateSettingRequest, $0.UpdateSettingResponse>(
          '/itcmdr.Itcmdr/UpdateSetting',
          ($0.UpdateSettingRequest value) => value.writeToBuffer(),
          $0.UpdateSettingResponse.fromBuffer);
  static final _$watchUpdates =
      $grpc.ClientMethod<$0.WatchUpdatesRequest, $0.Update>(
          '/itcmdr.Itcmdr/WatchUpdates',
          ($0.WatchUpdatesRequest value) => value.writeToBuffer(),
          $0.Update.fromBuffer);
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
    $addMethod($grpc.ServiceMethod<$0.GetMetricsRequest, $0.GetMetricsResponse>(
        'GetMetrics',
        getMetrics_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.GetMetricsRequest.fromBuffer(value),
        ($0.GetMetricsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ListExceptionsRequest,
            $0.ListExceptionsResponse>(
        'ListExceptions',
        listExceptions_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.ListExceptionsRequest.fromBuffer(value),
        ($0.ListExceptionsResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.SaveExceptionRequest, $0.SaveExceptionResponse>(
            'SaveException',
            saveException_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.SaveExceptionRequest.fromBuffer(value),
            ($0.SaveExceptionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ExpireExceptionRequest,
            $0.ExpireExceptionResponse>(
        'ExpireException',
        expireException_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.ExpireExceptionRequest.fromBuffer(value),
        ($0.ExpireExceptionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.SearchLogsRequest, $0.SearchLogsResponse>(
        'SearchLogs',
        searchLogs_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.SearchLogsRequest.fromBuffer(value),
        ($0.SearchLogsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ListAuditRequest, $0.ListAuditResponse>(
        'ListAudit',
        listAudit_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ListAuditRequest.fromBuffer(value),
        ($0.ListAuditResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.CreateBackupRequest, $0.CreateBackupResponse>(
            'CreateBackup',
            createBackup_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.CreateBackupRequest.fromBuffer(value),
            ($0.CreateBackupResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.ListBackupsRequest, $0.ListBackupsResponse>(
            'ListBackups',
            listBackups_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.ListBackupsRequest.fromBuffer(value),
            ($0.ListBackupsResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.VerifyBackupRequest, $0.VerifyBackupResponse>(
            'VerifyBackup',
            verifyBackup_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.VerifyBackupRequest.fromBuffer(value),
            ($0.VerifyBackupResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.GetSettingsRequest, $0.GetSettingsResponse>(
            'GetSettings',
            getSettings_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.GetSettingsRequest.fromBuffer(value),
            ($0.GetSettingsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.UpdateSettingsRequest,
            $0.UpdateSettingsResponse>(
        'UpdateSettings',
        updateSettings_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.UpdateSettingsRequest.fromBuffer(value),
        ($0.UpdateSettingsResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.ListSettingsRequest, $0.ListSettingsResponse>(
            'ListSettings',
            listSettings_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.ListSettingsRequest.fromBuffer(value),
            ($0.ListSettingsResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.UpdateSettingRequest, $0.UpdateSettingResponse>(
            'UpdateSetting',
            updateSetting_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.UpdateSettingRequest.fromBuffer(value),
            ($0.UpdateSettingResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.WatchUpdatesRequest, $0.Update>(
        'WatchUpdates',
        watchUpdates_Pre,
        false,
        true,
        ($core.List<$core.int> value) =>
            $0.WatchUpdatesRequest.fromBuffer(value),
        ($0.Update value) => value.writeToBuffer()));
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

  $async.Future<$0.GetMetricsResponse> getMetrics_Pre($grpc.ServiceCall $call,
      $async.Future<$0.GetMetricsRequest> $request) async {
    return getMetrics($call, await $request);
  }

  $async.Future<$0.GetMetricsResponse> getMetrics(
      $grpc.ServiceCall call, $0.GetMetricsRequest request);

  $async.Future<$0.ListExceptionsResponse> listExceptions_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListExceptionsRequest> $request) async {
    return listExceptions($call, await $request);
  }

  $async.Future<$0.ListExceptionsResponse> listExceptions(
      $grpc.ServiceCall call, $0.ListExceptionsRequest request);

  $async.Future<$0.SaveExceptionResponse> saveException_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.SaveExceptionRequest> $request) async {
    return saveException($call, await $request);
  }

  $async.Future<$0.SaveExceptionResponse> saveException(
      $grpc.ServiceCall call, $0.SaveExceptionRequest request);

  $async.Future<$0.ExpireExceptionResponse> expireException_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ExpireExceptionRequest> $request) async {
    return expireException($call, await $request);
  }

  $async.Future<$0.ExpireExceptionResponse> expireException(
      $grpc.ServiceCall call, $0.ExpireExceptionRequest request);

  $async.Future<$0.SearchLogsResponse> searchLogs_Pre($grpc.ServiceCall $call,
      $async.Future<$0.SearchLogsRequest> $request) async {
    return searchLogs($call, await $request);
  }

  $async.Future<$0.SearchLogsResponse> searchLogs(
      $grpc.ServiceCall call, $0.SearchLogsRequest request);

  $async.Future<$0.ListAuditResponse> listAudit_Pre($grpc.ServiceCall $call,
      $async.Future<$0.ListAuditRequest> $request) async {
    return listAudit($call, await $request);
  }

  $async.Future<$0.ListAuditResponse> listAudit(
      $grpc.ServiceCall call, $0.ListAuditRequest request);

  $async.Future<$0.CreateBackupResponse> createBackup_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.CreateBackupRequest> $request) async {
    return createBackup($call, await $request);
  }

  $async.Future<$0.CreateBackupResponse> createBackup(
      $grpc.ServiceCall call, $0.CreateBackupRequest request);

  $async.Future<$0.ListBackupsResponse> listBackups_Pre($grpc.ServiceCall $call,
      $async.Future<$0.ListBackupsRequest> $request) async {
    return listBackups($call, await $request);
  }

  $async.Future<$0.ListBackupsResponse> listBackups(
      $grpc.ServiceCall call, $0.ListBackupsRequest request);

  $async.Future<$0.VerifyBackupResponse> verifyBackup_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.VerifyBackupRequest> $request) async {
    return verifyBackup($call, await $request);
  }

  $async.Future<$0.VerifyBackupResponse> verifyBackup(
      $grpc.ServiceCall call, $0.VerifyBackupRequest request);

  $async.Future<$0.GetSettingsResponse> getSettings_Pre($grpc.ServiceCall $call,
      $async.Future<$0.GetSettingsRequest> $request) async {
    return getSettings($call, await $request);
  }

  $async.Future<$0.GetSettingsResponse> getSettings(
      $grpc.ServiceCall call, $0.GetSettingsRequest request);

  $async.Future<$0.UpdateSettingsResponse> updateSettings_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.UpdateSettingsRequest> $request) async {
    return updateSettings($call, await $request);
  }

  $async.Future<$0.UpdateSettingsResponse> updateSettings(
      $grpc.ServiceCall call, $0.UpdateSettingsRequest request);

  $async.Future<$0.ListSettingsResponse> listSettings_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListSettingsRequest> $request) async {
    return listSettings($call, await $request);
  }

  $async.Future<$0.ListSettingsResponse> listSettings(
      $grpc.ServiceCall call, $0.ListSettingsRequest request);

  $async.Future<$0.UpdateSettingResponse> updateSetting_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.UpdateSettingRequest> $request) async {
    return updateSetting($call, await $request);
  }

  $async.Future<$0.UpdateSettingResponse> updateSetting(
      $grpc.ServiceCall call, $0.UpdateSettingRequest request);

  $async.Stream<$0.Update> watchUpdates_Pre($grpc.ServiceCall $call,
      $async.Future<$0.WatchUpdatesRequest> $request) async* {
    yield* watchUpdates($call, await $request);
  }

  $async.Stream<$0.Update> watchUpdates(
      $grpc.ServiceCall call, $0.WatchUpdatesRequest request);
}
