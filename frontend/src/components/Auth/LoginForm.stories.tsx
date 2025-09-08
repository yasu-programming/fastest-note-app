import type { Meta, StoryObj } from '@storybook/react';
import { fn } from '@storybook/test';
import { LoginForm } from './LoginForm';

const meta: Meta<typeof LoginForm> = {
  title: 'Auth/LoginForm',
  component: LoginForm,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component: 'Login form component with email and password validation. Part of the authentication flow for the Fastest Note App.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    onSwitchToRegister: {
      description: 'Callback function triggered when user wants to switch to registration',
    },
  },
  args: {
    onSwitchToRegister: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Default login form ready for user input.',
      },
    },
  },
};

export const WithPlaceholders: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Login form showing typical input placeholders.',
      },
    },
  },
};

export const LoadingState: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Login form in loading state during authentication.',
      },
    },
  },
};