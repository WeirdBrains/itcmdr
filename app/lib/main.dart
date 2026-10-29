import 'package:flutter/material.dart';
import 'package:grpc/grpc_or_grpcweb.dart';
import 'proto/itcmdr.pbgrpc.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'itcmdr',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
      ),
      home: const OverviewPage(),
    );
  }
}

class OverviewPage extends StatefulWidget {
  const OverviewPage({super.key});

  @override
  State<OverviewPage> createState() => _OverviewPageState();
}

class _OverviewPageState extends State<OverviewPage> {
  late final ClientChannelBase _channel;
  late final ItcmdrClient _client;
  List<Machine> _machines = [];
  String? _error;

  @override
  void initState() {
    super.initState();
    _channel = createClientChannel();
    _client = ItcmdrClient(_channel);
    _refresh();
  }

  void _refresh() async {
    try {
      final resp = await _client.listMachines(ListMachinesRequest());
      setState(() {
        _machines = resp.machines;
        _error = null;
      });
    } catch (e) {
      setState(() {
        _error = e.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
        appBar: AppBar(title: const Text('itcmdr')),
        body: _error != null
            ? Center(child: Text('Server unavailable: \'$_error\''))
            : ListView.builder(
                itemCount: _machines.length,
                itemBuilder: (context, i) {
                  final m = _machines[i];
                  return ListTile(
                    leading: const Icon(Icons.desktop_windows),
                    title: Text(m.name),
                    subtitle:
                        Text('Incidents: \\${m.openIncidents}  Last run: \\${m.lastRunTs}'),
                    trailing: Text(m.status),
                  );
                },
              ),
        floatingActionButton:
            FloatingActionButton(onPressed: _refresh, child: const Icon(Icons.refresh)));
  }
}

ClientChannelBase createClientChannel() {
  // In browsers, this selects a GrpcWebClientChannel; on desktop, a native channel.
  const host = 'http://localhost:50051';
  return GrpcOrGrpcWebClientChannel.toSingleEndpoint(
    host: host,
    port: 0,
    transportSecure: false,
  );
}
