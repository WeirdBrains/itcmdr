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

  $grpc.ResponseFuture<$0.RunPluginResponse> runPlugin(
    $0.RunPluginRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$runPlugin, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListPluginRunsResponse> listPluginRuns(
    $0.ListPluginRunsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listPluginRuns, request, options: options);
  }

  $grpc.ResponseFuture<$0.GetPluginMetricsResponse> getPluginMetrics(
    $0.GetPluginMetricsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getPluginMetrics, request, options: options);
  }

  $grpc.ResponseFuture<$0.ScheduleResponse> createSchedule(
    $0.SaveScheduleRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$createSchedule, request, options: options);
  }

  $grpc.ResponseFuture<$0.ScheduleResponse> updateSchedule(
    $0.SaveScheduleRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$updateSchedule, request, options: options);
  }

  $grpc.ResponseFuture<$0.ScheduleResponse> deleteSchedule(
    $0.ScheduleRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$deleteSchedule, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListSchedulesResponse> listSchedules(
    $0.ListSchedulesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listSchedules, request, options: options);
  }

  $grpc.ResponseFuture<$0.ScheduleResponse> pauseSchedule(
    $0.ScheduleRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$pauseSchedule, request, options: options);
  }

  $grpc.ResponseFuture<$0.ScheduleResponse> resumeSchedule(
    $0.ScheduleRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$resumeSchedule, request, options: options);
  }

  $grpc.ResponseFuture<$0.RunScheduleNowResponse> runScheduleNow(
    $0.ScheduleRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$runScheduleNow, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListScheduleRunsResponse> listScheduleRuns(
    $0.ListScheduleRunsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listScheduleRuns, request, options: options);
  }

  $grpc.ResponseFuture<$0.ScheduleResponse> resetScheduleToDefault(
    $0.ScheduleRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$resetScheduleToDefault, request,
        options: options);
  }

  $grpc.ResponseFuture<$0.StageRestoreResponse> stageRestore(
    $0.StageRestoreRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$stageRestore, request, options: options);
  }

  $grpc.ResponseFuture<$0.GetRestoreStatusResponse> getRestoreStatus(
    $0.GetRestoreStatusRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getRestoreStatus, request, options: options);
  }

  $grpc.ResponseStream<$0.Update> watchUpdates(
    $0.WatchUpdatesRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createStreamingCall(
        _$watchUpdates, $async.Stream.fromIterable([request]),
        options: options);
  }

  /// v0.12.0
  $grpc.ResponseFuture<$0.RestoreKey> getRestoreKey(
    $0.GetRestoreKeyRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$getRestoreKey, request, options: options);
  }

  $grpc.ResponseFuture<$0.WhoAmIResponse> whoAmI(
    $0.WhoAmIRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$whoAmI, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListKeysResponse> listKeys(
    $0.ListKeysRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listKeys, request, options: options);
  }

  $grpc.ResponseFuture<$0.RegisterKeyResponse> registerKey(
    $0.RegisterKeyRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$registerKey, request, options: options);
  }

  $grpc.ResponseFuture<$0.RevokeKeyResponse> revokeKey(
    $0.RevokeKeyRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$revokeKey, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListPluginsResponse> listPlugins(
    $0.ListPluginsRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listPlugins, request, options: options);
  }

  $grpc.ResponseFuture<$0.PreviewScheduleResponse> previewSchedule(
    $0.SaveScheduleRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$previewSchedule, request, options: options);
  }

  /// v0.13.0
  $grpc.ResponseFuture<$0.InstallPluginResponse> installPlugin(
    $0.InstallPluginRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$installPlugin, request, options: options);
  }

  $grpc.ResponseFuture<$0.PluginActionResponse> enablePlugin(
    $0.PluginRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$enablePlugin, request, options: options);
  }

  $grpc.ResponseFuture<$0.PluginActionResponse> disablePlugin(
    $0.PluginRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$disablePlugin, request, options: options);
  }

  $grpc.ResponseFuture<$0.PluginActionResponse> removePlugin(
    $0.PluginRef request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$removePlugin, request, options: options);
  }

  $grpc.ResponseFuture<$0.ListQuarantineResponse> listQuarantine(
    $0.ListQuarantineRequest request, {
    $grpc.CallOptions? options,
  }) {
    return $createUnaryCall(_$listQuarantine, request, options: options);
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
  static final _$runPlugin =
      $grpc.ClientMethod<$0.RunPluginRequest, $0.RunPluginResponse>(
          '/itcmdr.Itcmdr/RunPlugin',
          ($0.RunPluginRequest value) => value.writeToBuffer(),
          $0.RunPluginResponse.fromBuffer);
  static final _$listPluginRuns =
      $grpc.ClientMethod<$0.ListPluginRunsRequest, $0.ListPluginRunsResponse>(
          '/itcmdr.Itcmdr/ListPluginRuns',
          ($0.ListPluginRunsRequest value) => value.writeToBuffer(),
          $0.ListPluginRunsResponse.fromBuffer);
  static final _$getPluginMetrics = $grpc.ClientMethod<
          $0.GetPluginMetricsRequest, $0.GetPluginMetricsResponse>(
      '/itcmdr.Itcmdr/GetPluginMetrics',
      ($0.GetPluginMetricsRequest value) => value.writeToBuffer(),
      $0.GetPluginMetricsResponse.fromBuffer);
  static final _$createSchedule =
      $grpc.ClientMethod<$0.SaveScheduleRequest, $0.ScheduleResponse>(
          '/itcmdr.Itcmdr/CreateSchedule',
          ($0.SaveScheduleRequest value) => value.writeToBuffer(),
          $0.ScheduleResponse.fromBuffer);
  static final _$updateSchedule =
      $grpc.ClientMethod<$0.SaveScheduleRequest, $0.ScheduleResponse>(
          '/itcmdr.Itcmdr/UpdateSchedule',
          ($0.SaveScheduleRequest value) => value.writeToBuffer(),
          $0.ScheduleResponse.fromBuffer);
  static final _$deleteSchedule =
      $grpc.ClientMethod<$0.ScheduleRef, $0.ScheduleResponse>(
          '/itcmdr.Itcmdr/DeleteSchedule',
          ($0.ScheduleRef value) => value.writeToBuffer(),
          $0.ScheduleResponse.fromBuffer);
  static final _$listSchedules =
      $grpc.ClientMethod<$0.ListSchedulesRequest, $0.ListSchedulesResponse>(
          '/itcmdr.Itcmdr/ListSchedules',
          ($0.ListSchedulesRequest value) => value.writeToBuffer(),
          $0.ListSchedulesResponse.fromBuffer);
  static final _$pauseSchedule =
      $grpc.ClientMethod<$0.ScheduleRef, $0.ScheduleResponse>(
          '/itcmdr.Itcmdr/PauseSchedule',
          ($0.ScheduleRef value) => value.writeToBuffer(),
          $0.ScheduleResponse.fromBuffer);
  static final _$resumeSchedule =
      $grpc.ClientMethod<$0.ScheduleRef, $0.ScheduleResponse>(
          '/itcmdr.Itcmdr/ResumeSchedule',
          ($0.ScheduleRef value) => value.writeToBuffer(),
          $0.ScheduleResponse.fromBuffer);
  static final _$runScheduleNow =
      $grpc.ClientMethod<$0.ScheduleRef, $0.RunScheduleNowResponse>(
          '/itcmdr.Itcmdr/RunScheduleNow',
          ($0.ScheduleRef value) => value.writeToBuffer(),
          $0.RunScheduleNowResponse.fromBuffer);
  static final _$listScheduleRuns = $grpc.ClientMethod<
          $0.ListScheduleRunsRequest, $0.ListScheduleRunsResponse>(
      '/itcmdr.Itcmdr/ListScheduleRuns',
      ($0.ListScheduleRunsRequest value) => value.writeToBuffer(),
      $0.ListScheduleRunsResponse.fromBuffer);
  static final _$resetScheduleToDefault =
      $grpc.ClientMethod<$0.ScheduleRef, $0.ScheduleResponse>(
          '/itcmdr.Itcmdr/ResetScheduleToDefault',
          ($0.ScheduleRef value) => value.writeToBuffer(),
          $0.ScheduleResponse.fromBuffer);
  static final _$stageRestore =
      $grpc.ClientMethod<$0.StageRestoreRequest, $0.StageRestoreResponse>(
          '/itcmdr.Itcmdr/StageRestore',
          ($0.StageRestoreRequest value) => value.writeToBuffer(),
          $0.StageRestoreResponse.fromBuffer);
  static final _$getRestoreStatus = $grpc.ClientMethod<
          $0.GetRestoreStatusRequest, $0.GetRestoreStatusResponse>(
      '/itcmdr.Itcmdr/GetRestoreStatus',
      ($0.GetRestoreStatusRequest value) => value.writeToBuffer(),
      $0.GetRestoreStatusResponse.fromBuffer);
  static final _$watchUpdates =
      $grpc.ClientMethod<$0.WatchUpdatesRequest, $0.Update>(
          '/itcmdr.Itcmdr/WatchUpdates',
          ($0.WatchUpdatesRequest value) => value.writeToBuffer(),
          $0.Update.fromBuffer);
  static final _$getRestoreKey =
      $grpc.ClientMethod<$0.GetRestoreKeyRequest, $0.RestoreKey>(
          '/itcmdr.Itcmdr/GetRestoreKey',
          ($0.GetRestoreKeyRequest value) => value.writeToBuffer(),
          $0.RestoreKey.fromBuffer);
  static final _$whoAmI =
      $grpc.ClientMethod<$0.WhoAmIRequest, $0.WhoAmIResponse>(
          '/itcmdr.Itcmdr/WhoAmI',
          ($0.WhoAmIRequest value) => value.writeToBuffer(),
          $0.WhoAmIResponse.fromBuffer);
  static final _$listKeys =
      $grpc.ClientMethod<$0.ListKeysRequest, $0.ListKeysResponse>(
          '/itcmdr.Itcmdr/ListKeys',
          ($0.ListKeysRequest value) => value.writeToBuffer(),
          $0.ListKeysResponse.fromBuffer);
  static final _$registerKey =
      $grpc.ClientMethod<$0.RegisterKeyRequest, $0.RegisterKeyResponse>(
          '/itcmdr.Itcmdr/RegisterKey',
          ($0.RegisterKeyRequest value) => value.writeToBuffer(),
          $0.RegisterKeyResponse.fromBuffer);
  static final _$revokeKey =
      $grpc.ClientMethod<$0.RevokeKeyRequest, $0.RevokeKeyResponse>(
          '/itcmdr.Itcmdr/RevokeKey',
          ($0.RevokeKeyRequest value) => value.writeToBuffer(),
          $0.RevokeKeyResponse.fromBuffer);
  static final _$listPlugins =
      $grpc.ClientMethod<$0.ListPluginsRequest, $0.ListPluginsResponse>(
          '/itcmdr.Itcmdr/ListPlugins',
          ($0.ListPluginsRequest value) => value.writeToBuffer(),
          $0.ListPluginsResponse.fromBuffer);
  static final _$previewSchedule =
      $grpc.ClientMethod<$0.SaveScheduleRequest, $0.PreviewScheduleResponse>(
          '/itcmdr.Itcmdr/PreviewSchedule',
          ($0.SaveScheduleRequest value) => value.writeToBuffer(),
          $0.PreviewScheduleResponse.fromBuffer);
  static final _$installPlugin =
      $grpc.ClientMethod<$0.InstallPluginRequest, $0.InstallPluginResponse>(
          '/itcmdr.Itcmdr/InstallPlugin',
          ($0.InstallPluginRequest value) => value.writeToBuffer(),
          $0.InstallPluginResponse.fromBuffer);
  static final _$enablePlugin =
      $grpc.ClientMethod<$0.PluginRef, $0.PluginActionResponse>(
          '/itcmdr.Itcmdr/EnablePlugin',
          ($0.PluginRef value) => value.writeToBuffer(),
          $0.PluginActionResponse.fromBuffer);
  static final _$disablePlugin =
      $grpc.ClientMethod<$0.PluginRef, $0.PluginActionResponse>(
          '/itcmdr.Itcmdr/DisablePlugin',
          ($0.PluginRef value) => value.writeToBuffer(),
          $0.PluginActionResponse.fromBuffer);
  static final _$removePlugin =
      $grpc.ClientMethod<$0.PluginRef, $0.PluginActionResponse>(
          '/itcmdr.Itcmdr/RemovePlugin',
          ($0.PluginRef value) => value.writeToBuffer(),
          $0.PluginActionResponse.fromBuffer);
  static final _$listQuarantine =
      $grpc.ClientMethod<$0.ListQuarantineRequest, $0.ListQuarantineResponse>(
          '/itcmdr.Itcmdr/ListQuarantine',
          ($0.ListQuarantineRequest value) => value.writeToBuffer(),
          $0.ListQuarantineResponse.fromBuffer);
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
    $addMethod($grpc.ServiceMethod<$0.RunPluginRequest, $0.RunPluginResponse>(
        'RunPlugin',
        runPlugin_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.RunPluginRequest.fromBuffer(value),
        ($0.RunPluginResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ListPluginRunsRequest,
            $0.ListPluginRunsResponse>(
        'ListPluginRuns',
        listPluginRuns_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.ListPluginRunsRequest.fromBuffer(value),
        ($0.ListPluginRunsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetPluginMetricsRequest,
            $0.GetPluginMetricsResponse>(
        'GetPluginMetrics',
        getPluginMetrics_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.GetPluginMetricsRequest.fromBuffer(value),
        ($0.GetPluginMetricsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.SaveScheduleRequest, $0.ScheduleResponse>(
        'CreateSchedule',
        createSchedule_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.SaveScheduleRequest.fromBuffer(value),
        ($0.ScheduleResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.SaveScheduleRequest, $0.ScheduleResponse>(
        'UpdateSchedule',
        updateSchedule_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.SaveScheduleRequest.fromBuffer(value),
        ($0.ScheduleResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ScheduleRef, $0.ScheduleResponse>(
        'DeleteSchedule',
        deleteSchedule_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ScheduleRef.fromBuffer(value),
        ($0.ScheduleResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.ListSchedulesRequest, $0.ListSchedulesResponse>(
            'ListSchedules',
            listSchedules_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.ListSchedulesRequest.fromBuffer(value),
            ($0.ListSchedulesResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ScheduleRef, $0.ScheduleResponse>(
        'PauseSchedule',
        pauseSchedule_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ScheduleRef.fromBuffer(value),
        ($0.ScheduleResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ScheduleRef, $0.ScheduleResponse>(
        'ResumeSchedule',
        resumeSchedule_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ScheduleRef.fromBuffer(value),
        ($0.ScheduleResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ScheduleRef, $0.RunScheduleNowResponse>(
        'RunScheduleNow',
        runScheduleNow_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ScheduleRef.fromBuffer(value),
        ($0.RunScheduleNowResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ListScheduleRunsRequest,
            $0.ListScheduleRunsResponse>(
        'ListScheduleRuns',
        listScheduleRuns_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.ListScheduleRunsRequest.fromBuffer(value),
        ($0.ListScheduleRunsResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ScheduleRef, $0.ScheduleResponse>(
        'ResetScheduleToDefault',
        resetScheduleToDefault_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ScheduleRef.fromBuffer(value),
        ($0.ScheduleResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.StageRestoreRequest, $0.StageRestoreResponse>(
            'StageRestore',
            stageRestore_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.StageRestoreRequest.fromBuffer(value),
            ($0.StageRestoreResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetRestoreStatusRequest,
            $0.GetRestoreStatusResponse>(
        'GetRestoreStatus',
        getRestoreStatus_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.GetRestoreStatusRequest.fromBuffer(value),
        ($0.GetRestoreStatusResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.WatchUpdatesRequest, $0.Update>(
        'WatchUpdates',
        watchUpdates_Pre,
        false,
        true,
        ($core.List<$core.int> value) =>
            $0.WatchUpdatesRequest.fromBuffer(value),
        ($0.Update value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.GetRestoreKeyRequest, $0.RestoreKey>(
        'GetRestoreKey',
        getRestoreKey_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.GetRestoreKeyRequest.fromBuffer(value),
        ($0.RestoreKey value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.WhoAmIRequest, $0.WhoAmIResponse>(
        'WhoAmI',
        whoAmI_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.WhoAmIRequest.fromBuffer(value),
        ($0.WhoAmIResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ListKeysRequest, $0.ListKeysResponse>(
        'ListKeys',
        listKeys_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.ListKeysRequest.fromBuffer(value),
        ($0.ListKeysResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.RegisterKeyRequest, $0.RegisterKeyResponse>(
            'RegisterKey',
            registerKey_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.RegisterKeyRequest.fromBuffer(value),
            ($0.RegisterKeyResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.RevokeKeyRequest, $0.RevokeKeyResponse>(
        'RevokeKey',
        revokeKey_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.RevokeKeyRequest.fromBuffer(value),
        ($0.RevokeKeyResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.ListPluginsRequest, $0.ListPluginsResponse>(
            'ListPlugins',
            listPlugins_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.ListPluginsRequest.fromBuffer(value),
            ($0.ListPluginsResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.SaveScheduleRequest, $0.PreviewScheduleResponse>(
            'PreviewSchedule',
            previewSchedule_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.SaveScheduleRequest.fromBuffer(value),
            ($0.PreviewScheduleResponse value) => value.writeToBuffer()));
    $addMethod(
        $grpc.ServiceMethod<$0.InstallPluginRequest, $0.InstallPluginResponse>(
            'InstallPlugin',
            installPlugin_Pre,
            false,
            false,
            ($core.List<$core.int> value) =>
                $0.InstallPluginRequest.fromBuffer(value),
            ($0.InstallPluginResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.PluginRef, $0.PluginActionResponse>(
        'EnablePlugin',
        enablePlugin_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.PluginRef.fromBuffer(value),
        ($0.PluginActionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.PluginRef, $0.PluginActionResponse>(
        'DisablePlugin',
        disablePlugin_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.PluginRef.fromBuffer(value),
        ($0.PluginActionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.PluginRef, $0.PluginActionResponse>(
        'RemovePlugin',
        removePlugin_Pre,
        false,
        false,
        ($core.List<$core.int> value) => $0.PluginRef.fromBuffer(value),
        ($0.PluginActionResponse value) => value.writeToBuffer()));
    $addMethod($grpc.ServiceMethod<$0.ListQuarantineRequest,
            $0.ListQuarantineResponse>(
        'ListQuarantine',
        listQuarantine_Pre,
        false,
        false,
        ($core.List<$core.int> value) =>
            $0.ListQuarantineRequest.fromBuffer(value),
        ($0.ListQuarantineResponse value) => value.writeToBuffer()));
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

  $async.Future<$0.RunPluginResponse> runPlugin_Pre($grpc.ServiceCall $call,
      $async.Future<$0.RunPluginRequest> $request) async {
    return runPlugin($call, await $request);
  }

  $async.Future<$0.RunPluginResponse> runPlugin(
      $grpc.ServiceCall call, $0.RunPluginRequest request);

  $async.Future<$0.ListPluginRunsResponse> listPluginRuns_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListPluginRunsRequest> $request) async {
    return listPluginRuns($call, await $request);
  }

  $async.Future<$0.ListPluginRunsResponse> listPluginRuns(
      $grpc.ServiceCall call, $0.ListPluginRunsRequest request);

  $async.Future<$0.GetPluginMetricsResponse> getPluginMetrics_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.GetPluginMetricsRequest> $request) async {
    return getPluginMetrics($call, await $request);
  }

  $async.Future<$0.GetPluginMetricsResponse> getPluginMetrics(
      $grpc.ServiceCall call, $0.GetPluginMetricsRequest request);

  $async.Future<$0.ScheduleResponse> createSchedule_Pre($grpc.ServiceCall $call,
      $async.Future<$0.SaveScheduleRequest> $request) async {
    return createSchedule($call, await $request);
  }

  $async.Future<$0.ScheduleResponse> createSchedule(
      $grpc.ServiceCall call, $0.SaveScheduleRequest request);

  $async.Future<$0.ScheduleResponse> updateSchedule_Pre($grpc.ServiceCall $call,
      $async.Future<$0.SaveScheduleRequest> $request) async {
    return updateSchedule($call, await $request);
  }

  $async.Future<$0.ScheduleResponse> updateSchedule(
      $grpc.ServiceCall call, $0.SaveScheduleRequest request);

  $async.Future<$0.ScheduleResponse> deleteSchedule_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.ScheduleRef> $request) async {
    return deleteSchedule($call, await $request);
  }

  $async.Future<$0.ScheduleResponse> deleteSchedule(
      $grpc.ServiceCall call, $0.ScheduleRef request);

  $async.Future<$0.ListSchedulesResponse> listSchedules_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListSchedulesRequest> $request) async {
    return listSchedules($call, await $request);
  }

  $async.Future<$0.ListSchedulesResponse> listSchedules(
      $grpc.ServiceCall call, $0.ListSchedulesRequest request);

  $async.Future<$0.ScheduleResponse> pauseSchedule_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.ScheduleRef> $request) async {
    return pauseSchedule($call, await $request);
  }

  $async.Future<$0.ScheduleResponse> pauseSchedule(
      $grpc.ServiceCall call, $0.ScheduleRef request);

  $async.Future<$0.ScheduleResponse> resumeSchedule_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.ScheduleRef> $request) async {
    return resumeSchedule($call, await $request);
  }

  $async.Future<$0.ScheduleResponse> resumeSchedule(
      $grpc.ServiceCall call, $0.ScheduleRef request);

  $async.Future<$0.RunScheduleNowResponse> runScheduleNow_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.ScheduleRef> $request) async {
    return runScheduleNow($call, await $request);
  }

  $async.Future<$0.RunScheduleNowResponse> runScheduleNow(
      $grpc.ServiceCall call, $0.ScheduleRef request);

  $async.Future<$0.ListScheduleRunsResponse> listScheduleRuns_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListScheduleRunsRequest> $request) async {
    return listScheduleRuns($call, await $request);
  }

  $async.Future<$0.ListScheduleRunsResponse> listScheduleRuns(
      $grpc.ServiceCall call, $0.ListScheduleRunsRequest request);

  $async.Future<$0.ScheduleResponse> resetScheduleToDefault_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.ScheduleRef> $request) async {
    return resetScheduleToDefault($call, await $request);
  }

  $async.Future<$0.ScheduleResponse> resetScheduleToDefault(
      $grpc.ServiceCall call, $0.ScheduleRef request);

  $async.Future<$0.StageRestoreResponse> stageRestore_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.StageRestoreRequest> $request) async {
    return stageRestore($call, await $request);
  }

  $async.Future<$0.StageRestoreResponse> stageRestore(
      $grpc.ServiceCall call, $0.StageRestoreRequest request);

  $async.Future<$0.GetRestoreStatusResponse> getRestoreStatus_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.GetRestoreStatusRequest> $request) async {
    return getRestoreStatus($call, await $request);
  }

  $async.Future<$0.GetRestoreStatusResponse> getRestoreStatus(
      $grpc.ServiceCall call, $0.GetRestoreStatusRequest request);

  $async.Stream<$0.Update> watchUpdates_Pre($grpc.ServiceCall $call,
      $async.Future<$0.WatchUpdatesRequest> $request) async* {
    yield* watchUpdates($call, await $request);
  }

  $async.Stream<$0.Update> watchUpdates(
      $grpc.ServiceCall call, $0.WatchUpdatesRequest request);

  $async.Future<$0.RestoreKey> getRestoreKey_Pre($grpc.ServiceCall $call,
      $async.Future<$0.GetRestoreKeyRequest> $request) async {
    return getRestoreKey($call, await $request);
  }

  $async.Future<$0.RestoreKey> getRestoreKey(
      $grpc.ServiceCall call, $0.GetRestoreKeyRequest request);

  $async.Future<$0.WhoAmIResponse> whoAmI_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.WhoAmIRequest> $request) async {
    return whoAmI($call, await $request);
  }

  $async.Future<$0.WhoAmIResponse> whoAmI(
      $grpc.ServiceCall call, $0.WhoAmIRequest request);

  $async.Future<$0.ListKeysResponse> listKeys_Pre($grpc.ServiceCall $call,
      $async.Future<$0.ListKeysRequest> $request) async {
    return listKeys($call, await $request);
  }

  $async.Future<$0.ListKeysResponse> listKeys(
      $grpc.ServiceCall call, $0.ListKeysRequest request);

  $async.Future<$0.RegisterKeyResponse> registerKey_Pre($grpc.ServiceCall $call,
      $async.Future<$0.RegisterKeyRequest> $request) async {
    return registerKey($call, await $request);
  }

  $async.Future<$0.RegisterKeyResponse> registerKey(
      $grpc.ServiceCall call, $0.RegisterKeyRequest request);

  $async.Future<$0.RevokeKeyResponse> revokeKey_Pre($grpc.ServiceCall $call,
      $async.Future<$0.RevokeKeyRequest> $request) async {
    return revokeKey($call, await $request);
  }

  $async.Future<$0.RevokeKeyResponse> revokeKey(
      $grpc.ServiceCall call, $0.RevokeKeyRequest request);

  $async.Future<$0.ListPluginsResponse> listPlugins_Pre($grpc.ServiceCall $call,
      $async.Future<$0.ListPluginsRequest> $request) async {
    return listPlugins($call, await $request);
  }

  $async.Future<$0.ListPluginsResponse> listPlugins(
      $grpc.ServiceCall call, $0.ListPluginsRequest request);

  $async.Future<$0.PreviewScheduleResponse> previewSchedule_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.SaveScheduleRequest> $request) async {
    return previewSchedule($call, await $request);
  }

  $async.Future<$0.PreviewScheduleResponse> previewSchedule(
      $grpc.ServiceCall call, $0.SaveScheduleRequest request);

  $async.Future<$0.InstallPluginResponse> installPlugin_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.InstallPluginRequest> $request) async {
    return installPlugin($call, await $request);
  }

  $async.Future<$0.InstallPluginResponse> installPlugin(
      $grpc.ServiceCall call, $0.InstallPluginRequest request);

  $async.Future<$0.PluginActionResponse> enablePlugin_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.PluginRef> $request) async {
    return enablePlugin($call, await $request);
  }

  $async.Future<$0.PluginActionResponse> enablePlugin(
      $grpc.ServiceCall call, $0.PluginRef request);

  $async.Future<$0.PluginActionResponse> disablePlugin_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.PluginRef> $request) async {
    return disablePlugin($call, await $request);
  }

  $async.Future<$0.PluginActionResponse> disablePlugin(
      $grpc.ServiceCall call, $0.PluginRef request);

  $async.Future<$0.PluginActionResponse> removePlugin_Pre(
      $grpc.ServiceCall $call, $async.Future<$0.PluginRef> $request) async {
    return removePlugin($call, await $request);
  }

  $async.Future<$0.PluginActionResponse> removePlugin(
      $grpc.ServiceCall call, $0.PluginRef request);

  $async.Future<$0.ListQuarantineResponse> listQuarantine_Pre(
      $grpc.ServiceCall $call,
      $async.Future<$0.ListQuarantineRequest> $request) async {
    return listQuarantine($call, await $request);
  }

  $async.Future<$0.ListQuarantineResponse> listQuarantine(
      $grpc.ServiceCall call, $0.ListQuarantineRequest request);
}
