'use client';

import React, { useState } from 'react';
import { LoginForm } from './LoginForm';
import { RegisterForm } from './RegisterForm';

type AuthMode = 'login' | 'register';

interface AuthContainerProps {
  initialMode?: AuthMode;
  onAuthSuccess?: () => void;
}

export const AuthContainer: React.FC<AuthContainerProps> = ({ 
  initialMode = 'login',
  onAuthSuccess 
}) => {
  const [mode, setMode] = useState<AuthMode>(initialMode);

  const switchToLogin = () => setMode('login');
  const switchToRegister = () => setMode('register');

  return (
    <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
      <div className="w-full max-w-md">
        {/* App Logo/Branding */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-blue-600 text-white rounded-full text-2xl font-bold mb-4">
            📝
          </div>
          <h1 className="text-3xl font-bold text-gray-900">Fastest Note App</h1>
          <p className="text-gray-600 mt-2">
            Lightning-fast note-taking with real-time sync
          </p>
        </div>

        {/* Auth Forms */}
        {mode === 'login' ? (
          <LoginForm onSwitchToRegister={switchToRegister} />
        ) : (
          <RegisterForm onSwitchToLogin={switchToLogin} />
        )}

        {/* Features Highlight */}
        <div className="mt-8 text-center">
          <div className="grid grid-cols-3 gap-4 text-sm text-gray-600">
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 bg-green-100 text-green-600 rounded-full flex items-center justify-center text-lg mb-2">
                ⚡
              </div>
              <span>Lightning Fast</span>
            </div>
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 bg-blue-100 text-blue-600 rounded-full flex items-center justify-center text-lg mb-2">
                🔄
              </div>
              <span>Real-time Sync</span>
            </div>
            <div className="flex flex-col items-center">
              <div className="w-8 h-8 bg-purple-100 text-purple-600 rounded-full flex items-center justify-center text-lg mb-2">
                📁
              </div>
              <span>Organized</span>
            </div>
          </div>
        </div>

        {/* Demo/Guest Access */}
        <div className="mt-6 text-center">
          <button
            type="button"
            className="text-sm text-gray-500 hover:text-gray-700 underline focus:outline-none"
          >
            Try Demo Version (No Account Required)
          </button>
        </div>
      </div>
    </div>
  );
};