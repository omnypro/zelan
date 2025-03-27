import { useEffect, useState } from 'react'

type FieldType = 'text' | 'number' | 'password' | 'checkbox' | 'textarea' | 'select'

type FieldOption = {
  label: string
  value: string
}

type FormField = {
  name: string
  label: string
  type: FieldType
  defaultValue: string | number | boolean
  placeholder?: string
  options?: FieldOption[]
  help?: string
}

type ConfigurationFormProps = {
  title: string
  description: string
  fields: FormField[]
  onSave?: (values: Record<string, any>) => void
}

export function ConfigurationForm({ title, description, fields, onSave }: ConfigurationFormProps) {
  const [values, setValues] = useState<Record<string, any>>({})
  const [isDirty, setIsDirty] = useState(false)
  
  // Initialize form values from default values
  useEffect(() => {
    const initialValues = fields.reduce(
      (acc, field) => ({
        ...acc,
        [field.name]: field.defaultValue,
      }),
      {}
    )
    
    setValues(initialValues)
    setIsDirty(false)
  }, [fields])
  
  const handleChange = (name: string, value: any) => {
    setValues((prev) => ({ ...prev, [name]: value }))
    setIsDirty(true)
  }
  
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (onSave) {
      onSave(values)
    }
    setIsDirty(false)
  }
  
  return (
    <div>
      <h2 className="text-xl font-bold">{title}</h2>
      <p className="text-gray-500 mb-6">{description}</p>
      
      <form onSubmit={handleSubmit} className="space-y-4">
        {fields.map((field) => (
          <div key={field.name} className="space-y-2">
            <div className="flex items-center justify-between">
              <label
                htmlFor={field.name}
                className={`text-sm font-medium ${field.type === 'checkbox' ? 'mb-0' : 'mb-1'}`}
              >
                {field.label}
              </label>
              
              {field.type === 'checkbox' && (
                <input
                  type="checkbox"
                  id={field.name}
                  name={field.name}
                  checked={values[field.name] || false}
                  onChange={(e) => handleChange(field.name, e.target.checked)}
                  className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                />
              )}
            </div>
            
            {field.type !== 'checkbox' && (
              <div>
                {field.type === 'textarea' ? (
                  <textarea
                    id={field.name}
                    name={field.name}
                    value={values[field.name] || ''}
                    onChange={(e) => handleChange(field.name, e.target.value)}
                    placeholder={field.placeholder}
                    className="w-full p-2 border rounded-md"
                    rows={4}
                  />
                ) : field.type === 'select' && field.options ? (
                  <select
                    id={field.name}
                    name={field.name}
                    value={values[field.name] || ''}
                    onChange={(e) => handleChange(field.name, e.target.value)}
                    className="w-full p-2 border rounded-md"
                  >
                    {field.options.map((option) => (
                      <option key={option.value} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </select>
                ) : (
                  <input
                    type={field.type}
                    id={field.name}
                    name={field.name}
                    value={values[field.name] || ''}
                    onChange={(e) => {
                      const value = field.type === 'number' 
                        ? parseFloat(e.target.value) 
                        : e.target.value;
                      handleChange(field.name, value);
                    }}
                    placeholder={field.placeholder}
                    className="w-full p-2 border rounded-md"
                  />
                )}
                
                {field.help && (
                  <p className="text-xs text-gray-500 mt-1">{field.help}</p>
                )}
              </div>
            )}
          </div>
        ))}
        
        <div className="pt-4 flex justify-end">
          <button
            type="submit"
            className={`px-4 py-2 bg-blue-600 text-white rounded-md ${!isDirty ? 'opacity-50 cursor-not-allowed' : 'hover:bg-blue-700'}`}
            disabled={!isDirty}
          >
            Save Changes
          </button>
        </div>
      </form>
    </div>
  )
}