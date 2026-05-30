import { createContext, useContext, useEffect, useState } from 'react';
import { useTraffic } from '@/hooks/traffic';

type Theme = 'dark' | 'light';

interface ThemeContextType {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

const ThemeContext = createContext<ThemeContextType | undefined>(undefined);

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const { prefs, updatePrefs } = useTraffic();
  const [theme, setThemeState] = useState<Theme>('dark');

  // 1. Initial Load from localStorage (faster than waiting for API)
  useEffect(() => {
    const savedTheme = localStorage.getItem('theme') as Theme;
    if (savedTheme) {
      setThemeState(savedTheme);
      document.documentElement.setAttribute('data-theme', savedTheme);
    }
  }, []);

  // 2. Sync with Global State when it arrives from Backend
  useEffect(() => {
    if (prefs.theme && prefs.theme !== theme) {
      setThemeState(prefs.theme);
      document.documentElement.setAttribute('data-theme', prefs.theme);
    }
  }, [prefs.theme]);

  const setTheme = (newTheme: Theme) => {
    setThemeState(newTheme);
    localStorage.setItem('theme', newTheme);
    document.documentElement.setAttribute('data-theme', newTheme);
    
    // Persist to backend
    updatePrefs({ ...prefs, theme: newTheme });
  };

  const toggleTheme = () => {
    const newTheme = theme === 'dark' ? 'light' : 'dark';
    setTheme(newTheme);
  };

  return (
    <ThemeContext.Provider value={{ theme, setTheme, toggleTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (context === undefined) {
    throw new Error('useTheme must be used within a ThemeProvider');
  }
  return context;
}
