%% tune_ekf.m — Herramienta de Calibración de Odometría (Olympus Rover)
%
% Este script permite ajustar los parámetros de ruido del EKF (Extended Kalman Filter)
% importando datos reales capturados con capture_rover_data.py.
%
% Parámetros a calibrar:
%   SIGMA2_THETA_BASE: Ruido base del giroscopio (estático)
%   K_RHO: Ruido del modelo de odometría (cuánto patinan las ruedas)
%   BETA_SLIP: Castigo al slip cuando la aceleración no coincide con encoders.

clear; clc; close all;

%% 1. Cargar Datos
filename = 'rover_capture_2026xxxx.csv'; % Ajustar al archivo generado
if ~exist(filename, 'file')
    fprintf('Error: No se encuentra el archivo %s. Ejecuta capture_rover_data.py primero.\n', filename);
    return;
end

T = readtable(filename);
t = T.local_ts - T.local_ts(1); % Tiempo relativo en segundos

%% 2. Parámetros del Rover (Ajustar según config.rs)
R_WHEEL = 0.050;      % 50 mm
B_EFF   = 0.280;      % 280 mm
TICKS_PER_REV = 20;   % Pulsos por vuelta
ENC_TO_METER = (2 * pi * R_WHEEL) / (3 * TICKS_PER_REV);

% Parámetros de Ruido (VALORES A CALIBRAR)
K_RHO = 1e-5;
SIGMA2_THETA_BASE = 1e-4;
ALPHA_SLIP = 0.008;

%% 3. Bucle de Simulación Kalman
% Inicialización de estados
x = 0; y = 0; theta = 0;
P = diag([0.1, 0.1, 0.05]); % Covarianza inicial
pos_hist = zeros(height(T), 3);
P_hist = zeros(height(T), 3); % Guardar diagonales de P

last_encL = T.enc_l(1);
last_encR = T.enc_r(1);

for i = 2:height(T)
    dt = t(i) - t(i-1);
    
    % Entradas (Encoders)
    deL = double(T.enc_l(i) - last_encL);
    deR = double(T.enc_r(i) - last_encR);
    dsL = deL * ENC_TO_METER;
    dsR = deR * ENC_TO_METER;
    ds = (dsL + dsR) / 2;
    dth = (dsR - dsL) / B_EFF;
    
    % Predicción de Estado (Dead Reckoning)
    mid_theta = theta + 0.5 * dth;
    x = x + ds * cos(mid_theta);
    y = y + ds * sin(mid_theta);
    theta = theta + dth;
    
    % --- ACTUALIZACIÓN DE COVARIANZA (Propuesta TFG) ---
    % F: Jacobiana del modelo de movimiento
    F = [1, 0, -ds*sin(mid_theta);
         0, 1,  ds*cos(mid_theta);
         0, 0,  1];
     
    % Q: Ruido de proceso
    sigma2_ds = K_RHO * abs(ds);
    sigma2_dth = SIGMA2_THETA_BASE + ALPHA_SLIP * abs(dth);
    
    Q = [0.25*cos(mid_theta)^2*sigma2_ds, 0.25*cos(mid_theta)*sin(mid_theta)*sigma2_ds, 0;
         0.25*cos(mid_theta)*sin(mid_theta)*sigma2_ds, 0.25*sin(mid_theta)^2*sigma2_ds, 0;
         0, 0, sigma2_dth];
     
    P = F * P * F' + Q;
    
    % Guardar resultados
    pos_hist(i, :) = [x, y, theta];
    P_hist(i, :) = diag(P)';
    
    last_encL = T.enc_l(i);
    last_encR = T.enc_r(i);
end

%% 4. Visualización de Resultados
figure('Name', 'Trayectoria Estimada (Odometría)');
plot(pos_hist(:,1), pos_hist(:,2), 'b', 'LineWidth', 1.5);
grid on; axis equal;
xlabel('X (m)'); ylabel('Y (m)');
title('Posición del Rover (X, Y)');

figure('Name', 'Incertidumbre (Covarianza)');
subplot(3,1,1); plot(t, P_hist(:,1)); ylabel('Var X (m^2)');
subplot(3,1,2); plot(t, P_hist(:,2)); ylabel('Var Y (m^2)');
subplot(3,1,3); plot(t, P_hist(:,3)); ylabel('Var Theta (rad^2)');
xlabel('Tiempo (s)');
title('Evolución de la Incertidumbre (P)');

fprintf('Simulación finalizada. Revisa las gráficas para ajustar K_RHO y SIGMA2_THETA.\n');
