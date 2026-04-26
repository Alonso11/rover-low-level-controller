%% tune_ekf.m — Validación y ajuste del EKF de odometría Olympus Rover
%
% PASOS:
%   1. Captura datos:
%      python3 scripts/capture_rover_data.py /dev/ttyUSB0 60
%      (60 s estático → solo MPU6050, sin encoders)
%
%   2. Ejecuta este script:
%      >> tune_ekf        (usa raw_*.csv más reciente)
%      >> tune_ekf('raw_20260426_120000.csv')
%
% OUTPUTS:
%   - Sección A: Señales crudas + bias estimado (del firmware)
%   - Sección B: Varianza Allan → ARW y bias instability del giroscopio
%   - Sección C: EKF completo (predict + update_gyro) con replay de datos
%   - Sección D: Trayectoria x/y/theta + evolución de covarianza P
%
% CALIBRACIÓN (sin encoders): solo update_gyro() actualiza theta.
%   predict() corre pero ds≈0 → P crece solo por sigma2_dth (ruido angular).
%   Con encoders: ajustar K_RHO y B_EFF midiendo trayectoria real.

function tune_ekf(raw_csv)

if nargin < 1
    files = dir('raw_*.csv');
    if isempty(files)
        error('No se encontró raw_*.csv. Ejecuta capture_rover_data.py primero.');
    end
    [~,idx] = max([files.datenum]);
    raw_csv = files(idx).name;
end
fprintf('Cargando: %s\n', raw_csv);

%% ── Parámetros del rover (igual que config.rs) ─────────────────────────────

R_WHEEL       = 0.050;          % m  — CALIBRAR con calibrador
B_EFF         = 0.280;          % m  — CALIBRAR con calibrador
TICKS_PER_REV = 20;             % pulsos/rev — CALIBRAR con encoder + 1 vuelta manual
ENC_TO_METER  = (2*pi*R_WHEEL) / (3*TICKS_PER_REV);

DT = 0.020;                     % s (20 ms, LOOP_MS en config.rs)

% Escalas IMU (igual que mpu6050.rs)
ACCEL_SCALE = 9.80665 / 16384;  % LSB → m/s²  (±2 g)
GYRO_SCALE  = (pi/180) / 131;   % LSB → rad/s (±250 °/s)

% Parámetros EKF — copiar de ekf.rs y ajustar aquí
SIGMA2_THETA_BASE = 1.0e-4;
ALPHA_SLIP        = 0.008;
K_RHO             = 1.0e-5;
BETA_SLIP         = 50.0;
GYRO_NOISE_DENSITY = 8.727e-5;  % rad/s/√Hz — de la hoja de datos MPU-6050
R_VEL = (GYRO_NOISE_DENSITY^2) * (1/DT) * DT^2;  % varianza medición giroscopio

%% ── A. Carga y conversión de unidades ────────────────────────────────────────

T = readtable(raw_csv, 'VariableNamingRule', 'preserve');
N = height(T);
fprintf('Muestras: %d  (~%.0f s @ 50 Hz)\n', N, N*DT);

t      = (0:N-1)' * DT;                % tiempo desde inicio (s)
ax     = double(T.ax_raw) * ACCEL_SCALE;   % m/s²
ay     = double(T.ay_raw) * ACCEL_SCALE;
az     = double(T.az_raw) * ACCEL_SCALE;
gz     = double(T.gz_raw) * GYRO_SCALE;    % rad/s
enc_l  = double(T.enc_l);
enc_r  = double(T.enc_r);

% Bias estático: media de los primeros 5 s (250 muestras)
n_cal = min(250, N);
bias_gz = mean(gz(1:n_cal));
bias_ax = mean(ax(1:n_cal));
bias_az = mean(az(1:n_cal));

gz_c = gz - bias_gz;
ax_c = ax - bias_ax;
az_c = az - (bias_az - 9.80665);   % restar bias, restaurar g

fprintf('\n── A. Bias estimado (firmware hace lo mismo con 50 muestras) ──\n');
fprintf('  bias_gz = %.6f rad/s  (%.4f °/s)\n', bias_gz, bias_gz*180/pi);
fprintf('  bias_ax = %.4f m/s²\n', bias_ax);
fprintf('  std(gz_corr) = %.6f rad/s\n', std(gz_c));

figure('Name','A — Señales IMU crudas');
subplot(2,1,1);
plot(t, gz*180/pi, 'b', t, gz_c*180/pi, 'r--');
legend('gz crudo','gz corregido'); ylabel('°/s'); grid on;
title('Giroscopio Z');
subplot(2,1,2);
plot(t, ax, 'b', t, az, 'r');
legend('ax','az'); ylabel('m/s²'); grid on; xlabel('t (s)');
title('Acelerómetro X, Z');

%% ── B. Varianza de Allan — caracterización de ruido del giroscopio ──────────
%
% Permite estimar:
%   ARW  (Angle Random Walk) → pendiente -1/2 en escala log-log → GYRO_NOISE_DENSITY
%   BI   (Bias Instability)  → mínimo de la curva
%
% Ref.: IEEE Std 1554-2005, §5.1 — Allan deviation for gyroscopes.

fprintf('\n── B. Varianza Allan ──────────────────────────────────────────\n');
[avar, tau] = allanvar_simple(gz_c, DT);
adev = sqrt(avar);

[~, idx_min] = min(adev);
arw  = adev(1) * sqrt(tau(1));           % ARW en rad/√s (intercepto a tau=1s)
bi   = adev(idx_min);                    % Bias instability en rad/s
tau_bi = tau(idx_min);

fprintf('  ARW  ≈ %.2e rad/√s  (%.2e °/√h)\n', arw, arw*180/pi*60);
fprintf('  Bias instability ≈ %.2e rad/s  @ tau=%.1f s\n', bi, tau_bi);
fprintf('  GYRO_NOISE_DENSITY actual: %.2e  (ARW×√Hz)\n', GYRO_NOISE_DENSITY);
fprintf('  → Valor medido:            %.2e\n', arw);
if abs(log10(arw) - log10(GYRO_NOISE_DENSITY)) > 0.5
    fprintf('  ADVERTENCIA: diferencia > ×3 — actualizar GYRO_NOISE_DENSITY en ekf.rs\n');
end

figure('Name','B — Varianza Allan');
loglog(tau, adev, 'b-o', 'MarkerSize', 4);
hold on;
xline(tau_bi, 'r--', sprintf('BI @ %.0f s',tau_bi));
yline(arw/sqrt(1), 'g--', 'ARW extrapolado');
xlabel('\tau (s)'); ylabel('\sigma(\tau) rad/s'); grid on;
title('Desviación de Allan — Giroscopio Z');
legend('Desviación Allan','Bias instability \tau','ARW ref');

%% ── C. EKF completo — predict() + update_gyro() ────────────────────────────
%
% Traducción directa de ekf.rs al mismo orden de operaciones.
% Con encoders desconectados: deL=deR=0 → predict solo propaga incertidumbre.
% El giroscopio corrige theta via update_gyro().

fprintf('\n── C. EKF replay ──────────────────────────────────────────────\n');

x = 0; y = 0; theta = 0; theta_prev = 0;
p00 = 0.01; p01 = 0; p02 = 0;
p11 = 0.01; p12 = 0; p22 = 0.09;

pos    = zeros(N, 3);   % [x, y, theta]
P_diag = zeros(N, 3);   % [p00, p11, p22]

for i = 2:N
    % ── Encoders (incremento desde muestra anterior) ─────────────────────
    deL = enc_l(i) - enc_l(i-1);
    deR = enc_r(i) - enc_r(i-1);
    dsL = deL * ENC_TO_METER;
    dsR = deR * ENC_TO_METER;
    ds  = 0.5*(dsR + dsL);
    dth = (dsR - dsL) / B_EFF;

    % ── predict() — mismo código que ekf.rs ──────────────────────────────
    mid = theta + 0.5*dth;
    cm = cos(mid); sm = sin(mid);
    theta_prev = theta;
    x     = x + ds*cm;
    y     = y + ds*sm;
    theta = wrap(theta + dth);

    % Q adaptivo
    v_enc   = ds / DT;
    v_accel = ax_c(i) * DT;
    slip = 0;
    if abs(v_enc) > 0.01
        slip = min(abs(v_enc - v_accel) / abs(v_enc), 1.0);
    end
    sigma2_ds  = K_RHO * abs(ds) * (1 + BETA_SLIP*slip^2);
    pitch      = atan2(ax_c(i), az_c(i));
    PITCH_THRESH = 0.14;
    Q_PITCH_MAX  = 0.06;
    q_pitch = 0;
    if abs(pitch) > PITCH_THRESH
        q_pitch = min((abs(pitch)-PITCH_THRESH)*3.0, Q_PITCH_MAX);
    end
    sigma2_dth = SIGMA2_THETA_BASE + ALPHA_SLIP*abs(dth) + q_pitch;

    % Jacobiana F (solo las derivadas no-triviales)
    f02 = -ds*sm;  f12 = ds*cm;

    % P⁻ = F·P·Fᵀ + Q
    fp00 = p00 + f02*p02;  fp01 = p01 + f02*p12;  fp02 = p02 + f02*p22;
    fp11 = p11 + f12*p12;  fp12 = p12 + f12*p22;

    q00 = 0.25*cm^2*sigma2_ds + 0.25*ds^2*sm^2*sigma2_dth;
    q01 = 0.25*cm*sm*sigma2_ds - 0.25*ds^2*sm*cm*sigma2_dth;
    q02 = -0.5*ds*sm*sigma2_dth;
    q11 = 0.25*sm^2*sigma2_ds + 0.25*ds^2*cm^2*sigma2_dth;
    q12 =  0.5*ds*cm*sigma2_dth;
    q22 = sigma2_dth;

    p00 = fp00 + fp02*f02 + q00;
    p01 = fp01 + fp02*f12 + q01;
    p02 = fp02             + q02;
    p11 = fp11 + fp12*f12 + q11;
    p12 = fp12             + q12;
    p22 = p22              + q22;

    % ── update_gyro() — mismo código que ekf.rs ──────────────────────────
    r_ang = R_VEL;                         % varianza de medición
    z     = gz_c(i) * DT;                  % medición: incremento angular
    nu    = z - wrap(theta - theta_prev);  % innovación

    ss = p22 + r_ang;                      % covarianza de innovación
    k0 = p02/ss;  k1 = p12/ss;  k2 = p22/ss;

    x     = x + k0*nu;
    y     = y + k1*nu;
    theta = wrap(theta + k2*nu);

    p00 = p00 - k0*p02;  p01 = p01 - k0*p12;  p02 = p02 - k0*p22;
    p11 = p11 - k1*p12;  p12 = p12 - k1*p22;  p22 = p22 - k2*p22;

    p00 = max(p00, 1e-9);
    p11 = max(p11, 1e-9);
    p22 = max(p22, r_ang);

    pos(i,:)    = [x, y, theta];
    P_diag(i,:) = [p00, p11, p22];
end

%% ── D. Visualización ─────────────────────────────────────────────────────────

figure('Name','C — Trayectoria EKF');
subplot(1,2,1);
plot(pos(:,1), pos(:,2), 'b', 'LineWidth', 1.5);
hold on; plot(0,0,'go','MarkerFaceColor','g'); grid on; axis equal;
xlabel('X (m)'); ylabel('Y (m)'); title('Posición X-Y (EKF)');

subplot(1,2,2);
plot(t, pos(:,3)*180/pi, 'r', 'LineWidth', 1.2);
grid on; xlabel('t (s)'); ylabel('°'); title('Orientación \theta');

figure('Name','D — Covarianza P');
subplot(3,1,1); plot(t, P_diag(:,1)); ylabel('P_{xx} (m²)'); grid on;
subplot(3,1,2); plot(t, P_diag(:,2)); ylabel('P_{yy} (m²)'); grid on;
subplot(3,1,3); plot(t, P_diag(:,3)); ylabel('P_{\theta\theta} (rad²)'); grid on;
xlabel('t (s)'); sgtitle('Evolución de covarianza P');

% Integración directa del giroscopio (sin EKF) para comparar
theta_gyro = cumsum(gz_c) * DT;
figure('Name','D — EKF vs integración pura');
plot(t, theta_gyro*180/pi, 'b--', t, pos(:,3)*180/pi, 'r');
legend('Integración pura gz','EKF update\_gyro'); grid on;
xlabel('t (s)'); ylabel('°');
title('Comparación \theta: EKF corrige la deriva del giroscopio');

fprintf('\nResumen del ajuste:\n');
fprintf('  SIGMA2_THETA_BASE = %.1e  (ruido angular base)\n', SIGMA2_THETA_BASE);
fprintf('  K_RHO             = %.1e  (ruido de odometría)\n', K_RHO);
fprintf('  θ final (EKF)     = %.3f°  (debería ser ≈0 si rover estático)\n', pos(end,3)*180/pi);
fprintf('  P_θθ final        = %.2e rad²  (incertidumbre acumulada)\n', P_diag(end,3));
fprintf('\nSi θ_final > 1° estático → revisar bias_gz o aumentar SIGMA2_THETA_BASE.\n');

end


%% ── Funciones auxiliares ─────────────────────────────────────────────────────

function th = wrap(th)
    while th >  pi; th = th - 2*pi; end
    while th <= -pi; th = th + 2*pi; end
end

function [avar, tau] = allanvar_simple(omega, dt)
% Varianza de Allan simplificada (overlapping) para una señal escalar.
% omega: señal de velocidad angular [rad/s]
% dt   : período de muestreo [s]
% Ref.: IEEE Std 1554-2005 §5.1
    N    = length(omega);
    maxM = floor(N/2);
    mvals = unique(round(logspace(0, log10(maxM), 100)));
    avar  = zeros(size(mvals));
    tau   = mvals * dt;
    theta = cumsum(omega) * dt;   % ángulo integrado
    for k = 1:length(mvals)
        m   = mvals(k);
        n   = N - 2*m;
        if n < 1; avar(k) = NaN; continue; end
        d   = theta(2*m+1:end) - 2*theta(m+1:N-m) + theta(1:n);
        avar(k) = sum(d.^2) / (2 * (tau(k)^2) * n);
    end
end
