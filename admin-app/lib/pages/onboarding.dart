// First run: enroll this device with the one-time code from an owner or admin. The 24 words
// are shown once; the app never stores them.
import 'package:flutter/material.dart';

import '../session.dart';

class OnboardingPage extends StatefulWidget {
  const OnboardingPage({super.key});
  @override
  State<OnboardingPage> createState() => _OnboardingPageState();
}

class _OnboardingPageState extends State<OnboardingPage> {
  final code = TextEditingController();
  bool keyBackup = true;
  bool busy = false;
  String? error;
  String? words;
  bool wroteDown = false;

  Future<void> _enroll(Session s) async {
    setState(() {
      busy = true;
      error = null;
    });
    try {
      final e = await s.backend.enroll(s.dir, code.text.trim(), keyBackup);
      setState(() => words = e.words);
      _pending = e.me;
    } catch (e) {
      setState(() => error = errorText(e));
    } finally {
      if (mounted) setState(() => busy = false);
    }
  }

  dynamic _pending;

  @override
  Widget build(BuildContext context) {
    final s = SessionScope.of(context);
    final w = words;
    return Scaffold(
      body: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: ListView(
            padding: const EdgeInsets.all(24),
            shrinkWrap: true,
            children: w == null
                ? [
                    Text(
                      'itcmdr Admin',
                      style: Theme.of(context).textTheme.headlineMedium,
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      'Paste the enrollment code your owner or admin gave you '
                      '(itcmdr-admin:…). It is single use and expires after 24 hours.',
                    ),
                    const SizedBox(height: 16),
                    TextField(
                      key: const Key('code'),
                      controller: code,
                      minLines: 2,
                      maxLines: 4,
                      decoration: const InputDecoration(
                        labelText: 'Enrollment code',
                        border: OutlineInputBorder(),
                      ),
                    ),
                    SwitchListTile(
                      key: const Key('backup'),
                      value: keyBackup,
                      onChanged: (v) => setState(() => keyBackup = v),
                      title: const Text('Store the encrypted key backup'),
                      subtitle: const Text(
                        'Encrypted with your 24 words on this device; '
                        'lets you restore on a new device.',
                      ),
                    ),
                    if (error != null)
                      Padding(
                        padding: const EdgeInsets.symmetric(vertical: 8),
                        child: Text(
                          error!,
                          key: const Key('error'),
                          style: TextStyle(
                            color: Theme.of(context).colorScheme.error,
                          ),
                        ),
                      ),
                    FilledButton(
                      key: const Key('enroll'),
                      onPressed: busy ? null : () => _enroll(s),
                      child: Text(busy ? 'Enrolling…' : 'Enroll this device'),
                    ),
                  ]
                : [
                    Text(
                      'Your 24 words',
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      'Write them down on paper now and keep them offline. They are '
                      'shown once and are the only way to recover your identity.',
                    ),
                    const SizedBox(height: 16),
                    Card(
                      child: Padding(
                        padding: const EdgeInsets.all(16),
                        child: Wrap(
                          spacing: 12,
                          runSpacing: 8,
                          children: [
                            for (final (i, x) in w.split(' ').indexed)
                              SizedBox(
                                width: 120,
                                child: Text(
                                  '${i + 1}. $x',
                                  style: const TextStyle(
                                    fontFamily: 'monospace',
                                  ),
                                ),
                              ),
                          ],
                        ),
                      ),
                    ),
                    CheckboxListTile(
                      key: const Key('wrote'),
                      value: wroteDown,
                      onChanged: (v) => setState(() => wroteDown = v ?? false),
                      title: const Text('I wrote them down'),
                    ),
                    FilledButton(
                      key: const Key('continue'),
                      onPressed: wroteDown
                          ? () async {
                              final m = _pending;
                              setState(() => words = null);
                              await s.signedIn(m);
                            }
                          : null,
                      child: const Text('Continue'),
                    ),
                  ],
          ),
        ),
      ),
    );
  }
}
